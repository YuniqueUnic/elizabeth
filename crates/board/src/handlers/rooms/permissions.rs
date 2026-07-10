use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use super::shared::{HandlerResult, room_info_from_room};
use crate::dto::rooms::{RoomView, UpdateRoomPermissionRequest};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::{Room, permission::RoomPermission};
use crate::permissions::PermissionBuilder;
use crate::repository::{IRoomRepository, RoomRepository};
use crate::state::AppState;
use crate::validation::RoomNameValidator;
use crate::websocket::types::RoomUpdateReason;

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/permissions",
    params(
        ("name" = String, Path, description = "房间 slug"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = UpdateRoomPermissionRequest,
    responses(
        (status = 200, description = "权限更新成功", body = RoomView),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效或已撤销"),
        (status = 403, description = "无更新权限"),
        (status = 404, description = "房间不存在")
    ),
    tag = "rooms"
)]
pub async fn update_permissions(
    Path(name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRoomPermissionRequest>,
) -> HandlerResult<RoomView> {
    RoomNameValidator::validate_identifier(&name)?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let token_perm = verified.claims.as_permission();
    if !verified.room.permission.can_delete() {
        return Err(AppError::permission_denied("Permission denied by room"));
    }
    if !token_perm.can_delete() {
        return Err(AppError::permission_denied("Permission denied by token"));
    }

    let repo = RoomRepository::new(app_state.db_pool.clone());
    let mut room = verified.room;
    let old_slug = room.slug.clone();
    let was_shareable = room.permission.can_share();
    room.permission = build_room_permission(&payload);
    update_slug_for_share_policy(&repo, &mut room, was_shareable).await?;

    let updated_room = repo
        .update_permissions_and_slug(&room)
        .await
        .map_err(|e| AppError::internal(format!("Failed to update room: {e}")))?;
    broadcast_permission_update(&app_state, &old_slug, &updated_room).await;
    app_state
        .connection_manager
        .disconnect_room(&old_slug, "Room permissions changed")
        .await;

    Ok(Json(RoomView::from(&updated_room)))
}

fn build_room_permission(payload: &UpdateRoomPermissionRequest) -> RoomPermission {
    let mut builder = PermissionBuilder::new();
    if payload.edit {
        builder = builder.with_edit();
    }
    if payload.share {
        builder = builder.with_share();
    }
    if payload.delete {
        builder = builder.with_delete();
    }
    builder.build()
}

async fn update_slug_for_share_policy(
    repo: &RoomRepository,
    room: &mut Room,
    was_shareable: bool,
) -> Result<(), AppError> {
    if room.permission.can_share() {
        room.slug = room.name.clone();
        return Ok(());
    }

    if !was_shareable && room.slug != room.name {
        return Ok(());
    }

    loop {
        let candidate = format!("{}_{}", room.name, Uuid::new_v4());
        let exists = repo
            .exists(&candidate)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?;
        if !exists {
            room.slug = candidate;
            return Ok(());
        }
    }
}

async fn broadcast_permission_update(app_state: &AppState, old_slug: &str, updated_room: &Room) {
    let broadcaster = app_state.broadcaster.clone();
    let room_info = room_info_from_room(updated_room);
    let new_slug = updated_room.slug.clone();
    let update_reason = if new_slug != old_slug {
        RoomUpdateReason::AddressChanged
    } else {
        RoomUpdateReason::PermissionsChanged
    };

    if let Err(e) = broadcaster
        .broadcast_room_update(old_slug, &room_info, update_reason.clone())
        .await
    {
        log::warn!("Failed to broadcast room update event (old slug): {}", e);
    }
    if new_slug != old_slug
        && let Err(e) = broadcaster
            .broadcast_room_update(&new_slug, &room_info, update_reason)
            .await
    {
        log::warn!("Failed to broadcast room update event (new slug): {}", e);
    }
}
