use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, header::CONTENT_LENGTH};
use futures::StreamExt;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::authz::{Authz, Resource};
use crate::constants::upload::MAX_MULTIPART_BODY_SIZE;
use crate::dto::content::UploadContentResponse;
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::UploadFileDescriptor;
use crate::models::room::role::Capability;
use crate::models::room::upload_file_policy::{
    is_safe_upload_file_name, upload_file_type_violation,
};
use crate::repository::{
    IRoomUploadReservationRepository, RoomContentRepository, RoomUploadReservationRepository,
};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::upload::{
    TempUpload, consume_upload_reservation, map_reservation_error, persist_staged_uploads,
    unique_upload_path,
};
use super::{HandlerResult, ensure_room_storage, room_id_or_error};

/// 单命令整文件上传：`curl -T file "$BASE/api/v1/rooms/{name}/files/{filename}"`。
/// 内部复用预留上传流程：按 Content-Length 预留 → 流式落盘 → 校验实际大小 → 持久化并核销。
#[utoipa::path(
    put,
    path = "/api/v1/rooms/{name}/files/{filename}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("filename" = String, Path, description = "目标文件名"),
        ("token" = Option<String>, Query, description = "有效的房间 token（也可用 Authorization 头携带）")
    ),
    responses(
        (status = 200, description = "上传成功", body = UploadContentResponse),
        (status = 400, description = "文件名不合法、请求体与 Content-Length 不符或文件类型被房间策略拒绝"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无上传权限"),
        (status = 404, description = "房间不存在"),
        (status = 413, description = "超出房间容量限制或请求体上限")
    ),
    tag = "content"
)]
pub async fn put_content(
    AxumPath((name, filename)): AxumPath<(String, String)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Body,
) -> HandlerResult<UploadContentResponse> {
    RoomNameValidator::validate_identifier(&name)?;

    if !is_safe_upload_file_name(&filename) {
        return Err(AppError::validation(format!(
            "Invalid file name: {filename}"
        )));
    }

    let content_length = content_length(&headers)?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = room_id_or_error(&verified.claims)?;
    authz.require(Capability::FileUpload, &Resource::Room { room_id })?;

    if !verified.room.upload_file_type.permits(&filename) {
        return Err(AppError::validation(upload_file_type_violation(&filename)));
    }

    if content_length > MAX_MULTIPART_BODY_SIZE as i64 {
        return Err(AppError::payload_too_large(format!(
            "File exceeds maximum body size of {MAX_MULTIPART_BODY_SIZE} bytes"
        )));
    }

    let manifest = [UploadFileDescriptor {
        name: filename.clone(),
        size: content_length,
        mime: None,
        chunk_size: None,
        file_hash: None,
    }];
    let manifest_json = serde_json::to_string(&manifest)
        .map_err(|e| AppError::internal(format!("Serialize manifest failed: {e}")))?;

    let reservation_repo = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let (reservation, _) = reservation_repo
        .reserve_upload(
            &verified.room,
            &verified.claims.jti,
            &verified.claims.jti,
            &manifest_json,
            content_length,
            app_state.upload_reservation_ttl(),
        )
        .await
        .map_err(map_reservation_error)?;
    let reservation_id = reservation
        .id
        .ok_or_else(|| AppError::internal("Reservation id missing"))?;

    let storage_dir = ensure_room_storage(app_state.storage_root().as_ref(), room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to prepare storage directory: {e}")))?;
    let file_path = unique_upload_path(&storage_dir, &filename)?;

    let size = match write_body_to_file(body, &file_path).await {
        Ok(size) => size,
        Err(error) => {
            fs::remove_file(&file_path).await.ok();
            return Err(error);
        }
    };
    if size != content_length {
        fs::remove_file(&file_path).await.ok();
        reservation_repo
            .release_if_pending(reservation_id)
            .await
            .ok();
        return Err(AppError::validation(format!(
            "Body size {size} does not match Content-Length {content_length}"
        )));
    }

    let mime = mime_guess::from_path(&filename)
        .first_raw()
        .map(|m| m.to_string());
    let staged = [TempUpload {
        original_name: filename,
        path: file_path,
        size,
        mime,
    }];

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let (uploaded, actual_total) = persist_staged_uploads(
        &repository,
        &app_state,
        &name,
        room_id,
        &verified.claims.jti,
        &staged,
    )
    .await?;
    let current_size = consume_upload_reservation(
        &reservation_repo,
        reservation_id,
        room_id,
        &verified.claims.jti,
        actual_total,
        &staged,
    )
    .await?;

    Ok(Json(UploadContentResponse {
        uploaded,
        current_size,
    }))
}

async fn write_body_to_file(body: Body, file_path: &Path) -> Result<i64, AppError> {
    let mut file = fs::File::create(file_path)
        .await
        .map_err(|e| AppError::internal(format!("Cannot create file: {e}")))?;

    let mut size: i64 = 0;
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| AppError::validation(format!("Read upload body failed: {e}")))?;
        size += chunk.len() as i64;
        file.write_all(&chunk)
            .await
            .map_err(|e| AppError::internal(format!("Write file failed: {e}")))?;
    }
    file.flush()
        .await
        .map_err(|e| AppError::internal(format!("Flush file failed: {e}")))?;

    Ok(size)
}

fn content_length(headers: &HeaderMap) -> Result<i64, AppError> {
    let raw = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::validation("Content-Length header is required"))?;
    let length: i64 = raw
        .trim()
        .parse()
        .map_err(|_| AppError::validation(format!("Invalid Content-Length header: {raw}")))?;
    if length <= 0 {
        return Err(AppError::validation(
            "Content-Length must be greater than 0",
        ));
    }
    Ok(length)
}
