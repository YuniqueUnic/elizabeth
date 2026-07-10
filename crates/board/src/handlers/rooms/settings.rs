use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};

use super::shared::{HandlerResult, room_info_from_room};
use crate::dto::rooms::UpdateRoomSettingsRequest;
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::Room;
use crate::repository::{IRoomRepository, RoomRepository};
use crate::state::AppState;
use crate::validation::{PasswordValidator, RoomNameValidator};
use crate::websocket::types::RoomUpdateReason;

/// 更新房间设置
#[utoipa::path(
    put,
    path = "/api/v1/rooms/{name}/settings",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token，需要删除权限")
    ),
    request_body = UpdateRoomSettingsRequest,
    responses(
        (status = 200, description = "设置更新成功", body = Room),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效或已撤销"),
        (status = 403, description = "无更新权限"),
        (status = 404, description = "房间不存在")
    ),
    tag = "rooms"
)]
pub async fn update_room_settings(
    Path(name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRoomSettingsRequest>,
) -> HandlerResult<Room> {
    RoomNameValidator::validate_identifier(&name)?;
    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let token_perm = verified.claims.as_permission();

    if !verified.room.permission.can_delete() || !token_perm.can_delete() {
        return Err(AppError::permission_denied(
            "Insufficient permissions to update room settings",
        ));
    }

    let repo = RoomRepository::new(app_state.db_pool.clone());
    let mut room = verified.room;
    apply_validated_settings_payload(&mut room, payload, app_state.room_expiry_policy())?;

    let updated_room = repo
        .update(&room)
        .await
        .map_err(|e| AppError::internal(format!("Failed to update room settings: {e}")))?;
    broadcast_settings_update(app_state, updated_room.clone());

    Ok(Json(updated_room))
}

fn validate_settings_payload(
    payload: &UpdateRoomSettingsRequest,
    expiry_policy: &crate::config::RoomExpiryPolicy,
) -> Result<(), AppError> {
    if let Some(ref password) = payload.password
        && !password.is_empty()
    {
        PasswordValidator::validate_room_password(password)?;
    }
    if let Some(max_times) = payload.max_times_entered
        && max_times <= 0
    {
        return Err(AppError::validation(
            "max_times_entered must be greater than 0",
        ));
    }
    if let Some(max_size) = payload.max_size
        && max_size <= 0
    {
        return Err(AppError::validation("max_size must be greater than 0"));
    }
    if let Some(age_seconds) = payload.age_seconds
        && !expiry_policy.allows(age_seconds)
    {
        return Err(AppError::validation(
            "age_seconds must be one of the configured room expiry ages",
        ));
    }
    Ok(())
}

pub(crate) fn apply_validated_settings_payload(
    room: &mut Room,
    payload: UpdateRoomSettingsRequest,
    expiry_policy: &crate::config::RoomExpiryPolicy,
) -> Result<(), AppError> {
    validate_settings_payload(&payload, expiry_policy)?;
    apply_settings_payload(room, payload, expiry_policy)
}

fn apply_settings_payload(
    room: &mut Room,
    payload: UpdateRoomSettingsRequest,
    expiry_policy: &crate::config::RoomExpiryPolicy,
) -> Result<(), AppError> {
    if let Some(password) = payload.password {
        room.password = if password.is_empty() {
            None
        } else {
            Some(password)
        };
    }

    if let Some(age_seconds) = payload.age_seconds {
        room.expire_at = Some(
            expiry_policy
                .expire_at(chrono::Utc::now().naive_utc(), age_seconds)
                .ok_or_else(|| AppError::validation("Invalid room expiry age"))?,
        );
    }

    if let Some(max_times) = payload.max_times_entered {
        room.max_times_entered = max_times;
    }

    if let Some(max_size) = payload.max_size {
        room.max_size = max_size;
    }
    Ok(())
}

fn broadcast_settings_update(app_state: Arc<AppState>, updated_room: Room) {
    let broadcaster = app_state.broadcaster.clone();
    let room_info = room_info_from_room(&updated_room);
    let room_slug = updated_room.slug.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_room_update(&room_slug, &room_info, RoomUpdateReason::SettingsChanged)
            .await
        {
            log::warn!("Failed to broadcast room update event: {}", e);
        }
    });
}
