use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};

use crate::authz::{Authz, Resource};
use crate::dto::content::{
    RoomContentView, SetContentVisibilityRequest, SetContentVisibilityResponse,
};
use crate::errors::{AppError, AppResult};
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::content::ContentType;
use crate::models::room::role::Capability;
use crate::repository::{IRoomContentRepository, RoomContentRepository};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::{HandlerResult, room_id_or_error};

/// 消息型内容（Text）使用 msg 域能力，文件/链接使用 file 域能力。
fn visibility_capability_for(content_type: ContentType) -> Capability {
    if content_type == ContentType::Text {
        Capability::MsgVisibilityManage
    } else {
        Capability::FileVisibilityManage
    }
}

/// 隐藏内容的读取闸门：hidden 内容一律按 not found 对外，避免向无能力者确认存在性。
pub(crate) fn ensure_content_visible(
    authz: &Authz<'_>,
    content: &crate::models::content::RoomContent,
) -> AppResult<()> {
    if !content.hidden {
        return Ok(());
    }
    let covered = authz.permits(
        visibility_capability_for(content.content_type),
        &Resource::Content {
            room_id: content.room_id,
            content_type: content.content_type,
            created_by_jti: content.created_by_jti.as_deref(),
        },
    );
    if covered {
        Ok(())
    } else {
        Err(AppError::not_found("Content not found"))
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/rooms/{name}/contents/{content_id}/visibility",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("content_id" = i64, Path, description = "内容 ID"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = SetContentVisibilityRequest,
    responses(
        (status = 200, description = "可见性已更新", body = SetContentVisibilityResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "缺少 msg/file.visibility.manage 能力"),
        (status = 404, description = "房间或内容不存在")
    ),
    tag = "content"
)]
pub async fn set_content_visibility(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SetContentVisibilityRequest>,
) -> HandlerResult<SetContentVisibilityResponse> {
    RoomNameValidator::validate_identifier(&name)?;

    if content_id <= 0 {
        return Err(AppError::validation("Invalid content ID"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;

    let room_id = room_id_or_error(&verified.claims)?;
    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let mut content = repository
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {e}")))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    if content.room_id != room_id {
        return Err(AppError::not_found("Content not found"));
    }

    authz.require(
        visibility_capability_for(content.content_type),
        &Resource::Content {
            room_id,
            content_type: content.content_type,
            created_by_jti: content.created_by_jti.as_deref(),
        },
    )?;

    if content.hidden != payload.hidden {
        content.hidden = payload.hidden;
        content.updated_at = chrono::Utc::now().naive_utc();
        content = repository
            .update(&content)
            .await
            .map_err(|e| AppError::internal(format!("Update failed: {e}")))?;
    }

    let broadcaster = app_state.broadcaster.clone();
    let room_name = name.clone();
    let broadcast_content = content.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_content_updated(&room_name, &broadcast_content)
            .await
        {
            log::warn!("Failed to broadcast content updated event: {}", e);
        }
    });

    Ok(Json(SetContentVisibilityResponse {
        updated: RoomContentView::from(content),
    }))
}
