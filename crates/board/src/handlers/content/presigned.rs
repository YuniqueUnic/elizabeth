//! presigned 传输模式的提交端点。
//!
//! prepare 在鉴权与配额校验后签发直传 URL（见 `upload::prepare_upload`）；
//! 客户端直传完成后调用本端点核对对象大小、落内容记录并核销预留。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use chrono::Utc;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::authz::{Authz, Resource};
use crate::dto::content::UploadContentResponse;
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::content::RoomContent;
use crate::models::room::role::Capability;
use crate::models::room::upload_reservation::UploadFileDescriptor;
use crate::repository::{
    IRoomContentRepository, IRoomUploadReservationRepository, RoomContentRepository,
    RoomUploadReservationRepository,
};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::upload::broadcast_content_created;
use super::{HandlerResult, room_id_or_error};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CommitPresignedRequest {
    pub reservation_id: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/contents/presigned-commit",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = Option<String>, Query, description = "有效的房间 token（也可用 Authorization 头携带）")
    ),
    request_body = CommitPresignedRequest,
    responses(
        (status = 200, description = "提交成功，内容已落记录并核销预留", body = UploadContentResponse),
        (status = 400, description = "预留不存在/过期，或直传对象缺失、大小与清单不符"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无上传权限"),
        (status = 404, description = "房间不存在")
    ),
    tag = "content"
)]
pub async fn commit_presigned_uploads(
    AxumPath(name): AxumPath<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CommitPresignedRequest>,
) -> HandlerResult<UploadContentResponse> {
    RoomNameValidator::validate_identifier(&name)?;

    if payload.reservation_id <= 0 {
        return Err(AppError::validation("Invalid reservation id"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = room_id_or_error(&verified.claims)?;
    authz.require(Capability::FileUpload, &Resource::Room { room_id })?;

    let reservation_repo = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation = reservation_repo
        .fetch_by_id(payload.reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("Load reservation failed: {e}")))?
        .ok_or_else(|| AppError::validation("Reservation not found"))?;

    if reservation.room_id != room_id {
        return Err(AppError::permission_denied("Reservation not for this room"));
    }
    if reservation.owner_token_jti != verified.claims.jti {
        return Err(AppError::permission_denied("Reservation token mismatch"));
    }
    if reservation.expires_at < Utc::now().naive_utc() {
        reservation_repo
            .release_if_pending(payload.reservation_id)
            .await
            .ok();
        return Err(AppError::validation("Reservation expired"));
    }
    if reservation.consumed_at.is_some() {
        return Err(AppError::validation("Reservation already consumed"));
    }

    let files: Vec<UploadFileDescriptor> = serde_json::from_str(&reservation.file_manifest)
        .map_err(|e| AppError::internal(format!("Parse reservation manifest failed: {e}")))?;
    if files.is_empty() {
        return Err(AppError::validation("Reservation manifest empty"));
    }

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let mut uploaded = Vec::new();
    let mut actual_total: i64 = 0;
    for file in &files {
        let key = file
            .storage_key
            .clone()
            .ok_or_else(|| AppError::internal("Reservation manifest is missing storage key"))?;
        let actual_size = app_state.storage.object_size(&key).await.map_err(|e| {
            if matches!(e, crate::storage::StorageError::NotFound(_)) {
                AppError::validation(format!(
                    "Presigned upload missing for {}: the file must be PUT to the presigned URL before commit",
                    file.name
                ))
            } else {
                AppError::internal(format!("Inspect presigned upload failed: {e}"))
            }
        })?;
        if actual_size != file.size.unsigned_abs() {
            return Err(AppError::validation(format!(
                "Uploaded size mismatch for {}: expected {}, got {actual_size}",
                file.name, file.size
            )));
        }

        let saved = repository
            .create(&build_presigned_content(
                room_id,
                &verified.claims.jti,
                file,
                key,
            ))
            .await
            .map_err(|e| AppError::internal(format!("Persist content failed: {e}")))?;
        broadcast_content_created(app_state.clone(), name.clone(), saved.clone());
        uploaded.push(crate::dto::content::RoomContentView::from(saved));

        actual_total = actual_total
            .checked_add(file.size)
            .ok_or_else(|| AppError::internal("Total size overflow"))?;
    }

    let actual_manifest_json = serde_json::to_string(&files)
        .map_err(|e| AppError::internal(format!("Serialize actual manifest failed: {e}")))?;
    let updated_room = reservation_repo
        .consume_reservation(
            payload.reservation_id,
            room_id,
            &verified.claims.jti,
            actual_total,
            &actual_manifest_json,
        )
        .await
        .map_err(|e| AppError::internal(format!("Finalize reservation failed: {e}")))?;

    Ok(Json(UploadContentResponse {
        uploaded,
        current_size: updated_room.current_size,
    }))
}

fn build_presigned_content(
    room_id: i64,
    owner_jti: &str,
    file: &UploadFileDescriptor,
    key: String,
) -> RoomContent {
    let now = Utc::now().naive_utc();
    let mut content = RoomContent {
        id: None,
        room_id,
        content_type: crate::models::content::ContentType::File,
        text: None,
        url: None,
        path: None,
        hash: None,
        file_name: Some(file.name.clone()),
        size: None,
        mime_type: None,
        sequence_number: 0,
        created_by_jti: Some(owner_jti.to_string()),
        hidden: false,
        created_at: now,
        updated_at: now,
    };
    content.set_path(
        key,
        crate::models::content::ContentType::File,
        file.size,
        file.mime
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string()),
    );
    content
}
