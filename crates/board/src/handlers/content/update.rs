use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};

use crate::db::DbPool;
use crate::dto::content::{RoomContentView, UpdateContentRequest, UpdateContentResponse};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::content::RoomContent;
use crate::models::room::Room;
use crate::repository::{
    IRoomContentRepository, IRoomRepository, RoomContentRepository, RoomRepository,
};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::{ContentPermission, HandlerResult, ensure_permission, room_id_or_error};

#[utoipa::path(
    put,
    path = "/api/v1/rooms/{name}/contents/{content_id}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("content_id" = i64, Path, description = "内容 ID"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = UpdateContentRequest,
    responses(
        (status = 200, description = "更新成功", body = UpdateContentResponse),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无编辑权限"),
        (status = 404, description = "房间或内容不存在"),
        (status = 409, description = "版本冲突")
    ),
    tag = "content"
)]
pub async fn update_content(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateContentRequest>,
) -> HandlerResult<UpdateContentResponse> {
    RoomNameValidator::validate_identifier(&name)?;

    if content_id <= 0 {
        return Err(AppError::validation("Invalid content ID"));
    }

    if payload.text.is_none() && payload.url.is_none() {
        return Err(AppError::validation(
            "At least one of text or url must be provided",
        ));
    }

    let mut verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let existing_content = repository
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {e}")))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    if existing_content.room_id != room_id {
        return Err(AppError::permission_denied("Content not in this room"));
    }

    let saved_content = persist_updated_content(&repository, existing_content, payload).await?;
    verified.room = room_repo_update_if_content_size_changed(
        &verified.room,
        &saved_content,
        &app_state.db_pool,
    )
    .await?;

    broadcast_content_updated(app_state, name, saved_content.clone());

    Ok(Json(UpdateContentResponse {
        updated: RoomContentView::from(saved_content),
    }))
}

async fn persist_updated_content(
    repository: &RoomContentRepository,
    existing_content: RoomContent,
    payload: UpdateContentRequest,
) -> Result<RoomContent, AppError> {
    let mut updated_content = existing_content;
    updated_content.updated_at = chrono::Utc::now().naive_utc();

    if let Some(text) = payload.text {
        updated_content.set_text(text);
    }

    if let Some(url) = payload.url {
        updated_content.set_url(url, payload.mime_type.clone());
    }

    repository
        .update(&updated_content)
        .await
        .map_err(|e| AppError::internal(format!("Update failed: {e}")))
}

fn broadcast_content_updated(app_state: Arc<AppState>, room_name: String, content: RoomContent) {
    let broadcaster = app_state.broadcaster.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_content_updated(&room_name, &content)
            .await
        {
            log::warn!("Failed to broadcast content updated event: {}", e);
        }
    });
}

async fn room_repo_update_if_content_size_changed(
    room: &Room,
    content: &RoomContent,
    db_pool: &Arc<DbPool>,
) -> Result<Room, AppError> {
    let size_changed = match content.size {
        Some(content_size) => content_size != room.current_size,
        None => false,
    };

    if size_changed {
        let room_repo = RoomRepository::new(db_pool.clone());
        room_repo
            .update(room)
            .await
            .map_err(|e| AppError::internal(format!("Update room failed: {e}")))
    } else {
        Ok(room.clone())
    }
}
