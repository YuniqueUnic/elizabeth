//! 房间角色矩阵 CRUD（全部要求 `room.roles.manage` 能力）。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};

use super::shared::{HandlerResult, broadcast_room_update};
use crate::authz::{Authz, Resource, validate_grants};
use crate::dto::roles::{CreateRoleRequest, DeleteRoleResponse, RoleDefinition, UpdateRoleRequest};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::room::role::{
    Capability, Grant, RoomRole, grants_to_json, is_system_role_key, parse_grants_json,
};
use crate::repository::{IRoomRoleRepository, RoomRoleRepository};
use crate::state::AppState;
use crate::validation::{RoleKeyValidator, RoomNameValidator};
use crate::websocket::types::RoomUpdateReason;

#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}/roles",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    responses(
        (status = 200, description = "角色矩阵", body = [RoleDefinition]),
        (status = 401, description = "token 无效或已撤销"),
        (status = 403, description = "缺少 room.roles.manage 能力"),
        (status = 404, description = "房间不存在")
    ),
    tag = "rooms"
)]
pub async fn list_roles(
    Path(name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<RoleDefinition>> {
    RoomNameValidator::validate_identifier(&name)?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    authz.require(Capability::RoomRolesManage, &Resource::Room { room_id })?;

    let repo = RoomRoleRepository::new(app_state.db_pool.clone());
    let roles = repo
        .list_by_room(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list roles: {e}")))?;
    Ok(Json(roles.into_iter().map(to_definition).collect()))
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/roles",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = CreateRoleRequest,
    responses(
        (status = 200, description = "角色已创建", body = RoleDefinition),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效或已撤销"),
        (status = 403, description = "缺少 room.roles.manage 能力"),
        (status = 404, description = "房间不存在"),
        (status = 409, description = "角色 key 已存在或为系统保留字")
    ),
    tag = "rooms"
)]
pub async fn create_role(
    Path(name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateRoleRequest>,
) -> HandlerResult<RoleDefinition> {
    RoomNameValidator::validate_identifier(&name)?;
    let role_key = payload.role_key.trim().to_string();
    RoleKeyValidator::validate(&role_key)?;
    if is_system_role_key(&role_key) {
        return Err(AppError::conflict("Role key is reserved for system roles"));
    }
    let display_name = validate_display_name(&payload.display_name)?;
    validate_grants_or_400(&payload.capabilities)?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    authz.require(Capability::RoomRolesManage, &Resource::Room { room_id })?;

    let repo = RoomRoleRepository::new(app_state.db_pool.clone());
    if repo
        .find_by_key(room_id, &role_key)
        .await
        .map_err(|e| AppError::internal(format!("Failed to load role: {e}")))?
        .is_some()
    {
        return Err(AppError::conflict("Role key already exists"));
    }
    let created = repo
        .create(
            room_id,
            &role_key,
            &display_name,
            &grants_to_json(&payload.capabilities),
        )
        .await
        .map_err(|e| AppError::internal(format!("Failed to create role: {e}")))?;
    broadcast_room_update(&app_state, &verified.room, RoomUpdateReason::RolesChanged).await;
    Ok(Json(to_definition(created)))
}

#[utoipa::path(
    put,
    path = "/api/v1/rooms/{name}/roles/{role_key}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("role_key" = String, Path, description = "角色 key"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "角色已更新（立即对全房会话生效）", body = RoleDefinition),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效或已撤销"),
        (status = 403, description = "缺少 room.roles.manage 能力"),
        (status = 404, description = "房间或角色不存在")
    ),
    tag = "rooms"
)]
pub async fn update_role(
    Path((name, role_key)): Path<(String, String)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRoleRequest>,
) -> HandlerResult<RoleDefinition> {
    RoomNameValidator::validate_identifier(&name)?;
    RoleKeyValidator::validate(&role_key)?;
    let display_name = validate_display_name(&payload.display_name)?;
    validate_grants_or_400(&payload.capabilities)?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    authz.require(Capability::RoomRolesManage, &Resource::Room { room_id })?;

    let repo = RoomRoleRepository::new(app_state.db_pool.clone());
    let existing = repo
        .find_by_key(room_id, &role_key)
        .await
        .map_err(|e| AppError::internal(format!("Failed to load role: {e}")))?
        .ok_or_else(|| AppError::not_found("Room role"))?;
    let updated = repo
        .update(
            room_id,
            &existing.role_key,
            &display_name,
            &grants_to_json(&payload.capabilities),
        )
        .await
        .map_err(|e| AppError::internal(format!("Failed to update role: {e}")))?;
    broadcast_room_update(&app_state, &verified.room, RoomUpdateReason::RolesChanged).await;
    Ok(Json(to_definition(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/rooms/{name}/roles/{role_key}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("role_key" = String, Path, description = "角色 key"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    responses(
        (status = 200, description = "删除结果", body = DeleteRoleResponse),
        (status = 401, description = "token 无效或已撤销"),
        (status = 403, description = "缺少 room.roles.manage 能力"),
        (status = 404, description = "房间或角色不存在"),
        (status = 409, description = "系统角色不可删除，或角色是默认加入角色")
    ),
    tag = "rooms"
)]
pub async fn delete_role(
    Path((name, role_key)): Path<(String, String)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<DeleteRoleResponse> {
    RoomNameValidator::validate_identifier(&name)?;
    RoleKeyValidator::validate(&role_key)?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    authz.require(Capability::RoomRolesManage, &Resource::Room { room_id })?;

    if is_system_role_key(&role_key) {
        return Err(AppError::conflict("System roles cannot be deleted"));
    }
    if verified.room.default_role_key == role_key {
        return Err(AppError::conflict(
            "Role is the default join role; change the default first",
        ));
    }

    let repo = RoomRoleRepository::new(app_state.db_pool.clone());
    let affected_tokens = repo
        .count_active_tokens_with_role(room_id, &role_key)
        .await
        .map_err(|e| AppError::internal(format!("Failed to count role tokens: {e}")))?;
    let deleted = repo
        .delete(room_id, &role_key)
        .await
        .map_err(|e| AppError::internal(format!("Failed to delete role: {e}")))?;
    if !deleted {
        return Err(AppError::not_found("Room role"));
    }
    broadcast_room_update(&app_state, &verified.room, RoomUpdateReason::RolesChanged).await;
    Ok(Json(DeleteRoleResponse {
        deleted,
        affected_tokens,
    }))
}

fn to_definition(role: RoomRole) -> RoleDefinition {
    let capabilities = parse_grants_json(&role.capabilities).unwrap_or_default();
    RoleDefinition {
        role_key: role.role_key,
        display_name: role.display_name,
        capabilities,
        is_system: role.is_system,
    }
}

fn validate_display_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::validation(
            "Display name must be between 1 and 64 characters",
        ));
    }
    Ok(name.to_string())
}

fn validate_grants_or_400(grants: &[Grant]) -> Result<(), AppError> {
    validate_grants(grants).map_err(|e| AppError::validation(e.to_string()))
}
