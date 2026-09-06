use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};

use super::shared::{HandlerResult, apply_room_defaults};
use crate::authz::{Authz, Resource, load_role_table};
use crate::dto::rooms::{CreateRoomRequest, CreateRoomResponse, DeleteRoomResponse, RoomView};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::Room;
use crate::models::room::role::{Capability, ROLE_ADMIN};
use crate::repository::{IRoomRepository, RoomAccessRepository, RoomRepository};
use crate::state::AppState;
use crate::validation::{PasswordValidator, RoomNameValidator};

/// 创建房间
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    request_body = CreateRoomRequest,
    responses(
        (status = 200, description = "房间创建成功，同时返回创建者 admin 身份码", body = CreateRoomResponse),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn create(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateRoomRequest>,
) -> HandlerResult<CreateRoomResponse> {
    RoomNameValidator::validate(&name)?;
    if let Some(ref password) = payload.password {
        PasswordValidator::validate_room_password(password)?;
    }

    let repository = RoomRepository::new(app_state.db_pool.clone());
    let room = new_room_with_defaults(&app_state, name, payload.password).await?;
    let created_room = repository
        .create_if_absent(&room)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create room: {e}")))?
        .ok_or_else(|| AppError::conflict("Room already exists"))?;
    let (token, claims) = app_state
        .token_service()
        .issue(&created_room, ROLE_ADMIN)
        .map_err(|e| AppError::internal(format!("Failed to issue admin identity code: {e}")))?;
    let record = crate::models::RoomToken::new(
        claims.room_id,
        claims.jti.clone(),
        ROLE_ADMIN,
        claims.expires_at(),
    );
    let granted = RoomAccessRepository::new(app_state.db_pool.clone())
        .grant_new_session(
            claims.room_id,
            &record,
            None,
            chrono::Utc::now().naive_utc(),
        )
        .await
        .map_err(|e| AppError::internal(format!("Failed to persist admin identity code: {e}")))?;
    if !granted {
        return Err(AppError::internal(
            "Created room could not grant admin identity code",
        ));
    }
    let role_table = load_role_table(
        &app_state.roles_cache,
        &app_state.db_pool,
        claims.room_id,
        created_room.roles_version,
    )
    .await?;
    Ok(Json(CreateRoomResponse {
        room: RoomView::from(&created_room),
        token,
        claims,
        expires_at: record.expires_at,
        capabilities: role_table.grants(ROLE_ADMIN).unwrap_or_default().to_vec(),
    }))
}

/// 查找房间
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    responses(
        (status = 200, description = "房间信息；合法名称不存在时按部署默认配置创建", body = RoomView),
        (status = 410, description = "房间已过期"),
        (status = 403, description = "房间无法进入"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn find(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<RoomView> {
    RoomNameValidator::validate_identifier(&name)?;

    let repository = RoomRepository::new(app_state.db_pool.clone());
    if let Some(room) = resolve_existing_room(&repository, &name).await? {
        return Ok(room);
    }

    // Product contract: opening a valid room URL is a zero-step provisioning flow.
    // Only a true miss reaches this command path; expired, closed, entry-limited, or
    // reserved display names are resolved above and must never be silently replaced.
    RoomNameValidator::validate(&name)?;
    let room = new_room_with_defaults(&app_state, name.clone(), None).await?;
    match repository
        .create_if_absent(&room)
        .await
        .map_err(|e| AppError::internal(format!("Failed to auto-create room: {e}")))?
    {
        Some(created_room) => Ok(Json(RoomView::from(&created_room))),
        None => resolve_existing_room(&repository, &name)
            .await?
            .ok_or_else(|| AppError::internal("Concurrent room creation could not be resolved")),
    }
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
        return Err(AppError::room_expired(name));
    }

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    authz.require(Capability::RoomDelete, &Resource::Room { room_id })?;

    let deleted = app_state
        .services
        .room_lifecycle
        .delete_room(&app_state.connection_manager, room_id, &room.slug)
        .await
        .map_err(|e| AppError::internal(format!("Failed to delete room: {e}")))?;
    if !deleted {
        return Err(AppError::room_not_found(name));
    }
    Ok(Json(DeleteRoomResponse {
        message: "Room deleted successfully".to_string(),
    }))
}

async fn new_room_with_defaults(
    app_state: &AppState,
    name: String,
    requested_password: Option<String>,
) -> Result<Room, AppError> {
    let password = match requested_password {
        Some(password) if password.trim().is_empty() => None,
        Some(password) => Some(password),
        None => app_state.room_creation_defaults().password.clone(),
    };
    let password = match password {
        Some(password) => Some(
            app_state
                .room_password_service()
                .hash(password)
                .await
                .map_err(|e| AppError::internal(format!("Failed to protect room password: {e}")))?,
        ),
        None => None,
    };
    let mut room = Room::new(name, password);
    apply_room_defaults(&mut room, app_state)?;
    Ok(room)
}

async fn resolve_existing_room(
    repository: &RoomRepository,
    name: &str,
) -> Result<Option<Json<RoomView>>, AppError> {
    if let Some(room) = repository.find_by_name(name).await? {
        return ensure_room_enterable(room).map(Some);
    }
    if let Some(room) = repository.find_by_display_name(name).await? {
        return handle_display_name_match(name.to_string(), room).map(Some);
    }
    Ok(None)
}

fn ensure_room_enterable(room: Room) -> HandlerResult<RoomView> {
    if room.is_expired() {
        return Err(AppError::room_expired(room.slug));
    }
    if room.can_enter() {
        Ok(Json(RoomView::from(&room)))
    } else {
        Err(AppError::authentication("Room cannot be entered"))
    }
}

fn handle_display_name_match(name: String, room: Room) -> HandlerResult<RoomView> {
    if room.is_expired() {
        Err(AppError::room_expired(name))
    } else {
        Err(AppError::authentication("Room cannot be accessed"))
    }
}
