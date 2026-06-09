use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use utoipa::ToSchema;

use super::shared::{HandlerResult, apply_room_defaults};
use crate::dto::rooms::DeleteRoomResponse;
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::Room;
use crate::repository::{
    IRoomContentRepository, IRoomRepository, RoomContentRepository, RoomRepository,
};
use crate::state::AppState;
use crate::validation::{PasswordValidator, RoomNameValidator};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoomParams {
    password: Option<String>,
}

/// 创建房间
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("password" = Option<String>, Query, description = "房间密码")
    ),
    responses(
        (status = 200, description = "房间创建成功", body = Room),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn create(
    Path(name): Path<String>,
    Query(params): Query<CreateRoomParams>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Room> {
    RoomNameValidator::validate(&name)?;
    if let Some(ref password) = params.password {
        PasswordValidator::validate_room_password(password)?;
    }

    let repository = RoomRepository::new(app_state.db_pool.clone());
    if repository.exists(&name).await? {
        return Err(AppError::conflict("Room already exists"));
    }

    create_room_with_defaults(&repository, &app_state, name, params.password).await
}

/// 查找房间
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    responses(
        (status = 200, description = "房间信息", body = Room),
        (status = 403, description = "房间无法进入"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn find(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Room> {
    RoomNameValidator::validate_identifier(&name)?;

    let repository = RoomRepository::new(app_state.db_pool.clone());
    if let Some(room) = repository.find_by_name(&name).await? {
        return ensure_room_enterable(room);
    }

    if let Some(room) = repository.find_by_display_name(&name).await? {
        return handle_display_name_match(&repository, &app_state, name, room).await;
    }

    create_room_with_defaults(&repository, &app_state, name, None).await
}

/// 删除房间
#[utoipa::path(
    delete,
    path = "/api/v1/rooms/{name}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "管理员访问令牌，需具备删除权限")
    ),
    responses(
        (status = 200, description = "房间删除成功", body = DeleteRoomResponse),
        (status = 404, description = "房间不存在"),
        (status = 410, description = "房间已过期"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn delete(
    Path(name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<DeleteRoomResponse> {
    RoomNameValidator::validate_identifier(&name)?;

    let repository = RoomRepository::new(app_state.db_pool.clone());
    let room = repository
        .find_by_name(&name)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::room_not_found(&name))?;

    if room.is_expired() {
        return Err(AppError::authentication("Room has expired"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    if !verified.claims.as_permission().can_delete() {
        return Err(AppError::permission_denied(
            "Insufficient permissions to delete room",
        ));
    }

    delete_room_contents(&app_state, &room).await?;
    delete_room_record(&repository, &name).await
}

async fn create_room_with_defaults(
    repository: &RoomRepository,
    app_state: &AppState,
    name: String,
    password: Option<String>,
) -> HandlerResult<Room> {
    let mut room = Room::new(name, password);
    apply_room_defaults(&mut room, app_state);
    let created_room = repository
        .create(&room)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create room: {}", e)))?;
    Ok(Json(created_room))
}

fn ensure_room_enterable(room: Room) -> HandlerResult<Room> {
    if room.is_expired() {
        return Err(AppError::authentication("Room has expired"));
    }
    if room.can_enter() {
        Ok(Json(room))
    } else {
        Err(AppError::authentication("Room cannot be entered"))
    }
}

async fn handle_display_name_match(
    repository: &RoomRepository,
    app_state: &AppState,
    name: String,
    room: Room,
) -> HandlerResult<Room> {
    if room.slug != room.name {
        let lock_duration = app_state.config.room.share_disabled_lock_duration;
        let now = chrono::Utc::now().naive_utc();
        let lock_expired = (now - room.updated_at).num_seconds() >= lock_duration;
        if lock_expired {
            release_expired_private_room_name(repository, app_state, name, room, now).await
        } else {
            Err(AppError::authentication("Room cannot be accessed"))
        }
    } else {
        Err(AppError::authentication("Room cannot be accessed"))
    }
}

async fn release_expired_private_room_name(
    repository: &RoomRepository,
    app_state: &AppState,
    name: String,
    mut room: Room,
    now: chrono::NaiveDateTime,
) -> HandlerResult<Room> {
    room.name = room.slug.clone();
    room.updated_at = now;
    repository
        .update(&room)
        .await
        .map_err(|e| AppError::internal(format!("Failed to release expired room name: {}", e)))?;
    logrs::info!(
        "Instantly released expired private room name '{}' by renaming it to slug '{}'",
        name,
        room.slug
    );

    create_room_with_defaults(repository, app_state, name, None).await
}

async fn delete_room_contents(app_state: &Arc<AppState>, room: &Room) -> Result<(), AppError> {
    let Some(room_id) = room.id else {
        return Ok(());
    };

    let content_repo = RoomContentRepository::new(app_state.db_pool.clone());
    let contents = content_repo
        .list_by_room(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list room contents: {}", e)))?;

    for content in &contents {
        if let Some(path) = &content.path {
            tokio::fs::remove_file(path).await.ok();
        }
    }

    content_repo
        .delete_by_room_id(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to delete room contents: {}", e)))?;
    Ok(())
}

async fn delete_room_record(
    repository: &RoomRepository,
    name: &str,
) -> HandlerResult<DeleteRoomResponse> {
    match repository.delete(name).await {
        Ok(true) => {
            logrs::info!(
                "Room {} deleted successfully with all content cleaned up",
                name
            );
            Ok(Json(DeleteRoomResponse {
                message: "Room deleted successfully".to_string(),
            }))
        }
        Ok(false) => Err(AppError::room_not_found(name)),
        Err(e) => Err(AppError::internal(format!("Failed to delete room: {}", e))),
    }
}
