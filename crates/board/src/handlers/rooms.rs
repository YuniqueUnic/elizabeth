use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum_responses::http::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

use super::verify_room_token;
use crate::models::{Room, RoomToken, permission::RoomPermission};
use crate::repository::{
    IRoomRepository, IRoomTokenRepository, SqliteRoomRepository, SqliteRoomTokenRepository,
};
use crate::services::RoomTokenClaims;
use crate::state::{AppState, RoomDefaults};

type HandlerResult<T> = Result<Json<T>, HttpResponse>;

fn apply_room_defaults(room: &mut Room, defaults: &RoomDefaults) {
    room.max_size = defaults.max_size;
    room.max_times_entered = defaults.max_times_entered;
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoomParams {
    password: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct IssueTokenRequest {
    /// 房间密码，如果房间设置了密码，则必须填写
    pub password: Option<String>,
    /// 已有的房间 token，可用于在无需密码的情况下续签
    pub token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IssueTokenResponse {
    pub token: String,
    pub claims: RoomTokenClaims,
    pub expires_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidateTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ValidateTokenResponse {
    pub claims: RoomTokenClaims,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenQuery {
    pub token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoomPermissionRequest {
    #[serde(default)]
    pub edit: bool,
    #[serde(default)]
    pub share: bool,
    #[serde(default)]
    pub delete: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeTokenResponse {
    pub revoked: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoomTokenView {
    pub jti: String,
    pub expires_at: NaiveDateTime,
    pub revoked_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

impl From<RoomToken> for RoomTokenView {
    fn from(value: RoomToken) -> Self {
        Self {
            jti: value.jti,
            expires_at: value.expires_at,
            revoked_at: value.revoked_at,
            created_at: value.created_at,
        }
    }
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
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let repository = SqliteRoomRepository::new(app_state.db_pool.clone());

    if repository.exists(&name).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Database error: {}", e))
    })? {
        return Err(HttpResponse::BadRequest().message("Room already exists"));
    }

    let mut room = Room::new(name.clone(), params.password);
    apply_room_defaults(&mut room, &app_state.room_defaults);
    let created_room = repository.create(&room).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to create room: {}", e))
    })?;

    Ok(Json(created_room))
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
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let repository = SqliteRoomRepository::new(app_state.db_pool.clone());

    match repository.find_by_name(&name).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Database error: {}", e))
    })? {
        Some(room) => {
            if room.can_enter() {
                Ok(Json(room))
            } else {
                Err(HttpResponse::Forbidden().message("Room cannot be entered"))
            }
        }
        None => {
            // 如果房间不存在，判断是否存在同名但不同 slug 的房间
            if repository
                .find_by_display_name(&name)
                .await
                .map_err(|e| {
                    HttpResponse::InternalServerError().message(format!("Database error: {e}"))
                })?
                .is_some()
            {
                return Err(HttpResponse::Forbidden().message("Room cannot be accessed"));
            }

            let mut new_room = Room::new(name.clone(), None);
            apply_room_defaults(&mut new_room, &app_state.room_defaults);
            let created_room = repository.create(&new_room).await.map_err(|e| {
                HttpResponse::InternalServerError().message(format!("Failed to create room: {}", e))
            })?;
            Ok(Json(created_room))
        }
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
        (status = 200, description = "房间删除成功"),
        (status = 404, description = "房间不存在"),
        (status = 410, description = "房间已过期"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn delete(
    Path(name): Path<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if name.is_empty() {
        return HttpResponse::BadRequest()
            .message("Invalid room name")
            .into_response();
    }

    let repository = SqliteRoomRepository::new(app_state.db_pool.clone());

    // 首先检查房间是否存在和过期
    match repository.find_by_name(&name).await {
        Ok(Some(room)) => {
            // 检查房间是否过期
            if room.is_expired() {
                return HttpResponse::Gone()
                    .message("Room has expired")
                    .into_response();
            }
        }
        Ok(None) => {
            return HttpResponse::NotFound()
                .message("Room not found")
                .into_response();
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .message(format!("Database error: {}", e))
                .into_response();
        }
    }

    let verified = match verify_room_token(app_state.clone(), &name, &query.token).await {
        Ok(verified) => verified,
        Err(err) => return err.into_response(),
    };

    if !verified.claims.as_permission().can_delete() {
        return HttpResponse::Forbidden()
            .message("Permission denied by token")
            .into_response();
    }

    // 房间存在且未过期，执行删除
    match repository.delete(&name).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to delete room: {}", e))
    }) {
        Ok(true) => HttpResponse::Ok()
            .message("Room deleted successfully")
            .into_response(),
        Ok(false) => HttpResponse::NotFound()
            .message("Room not found")
            .into_response(),
        Err(e) => e.into_response(),
    }
}

/// 签发房间访问凭证
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/tokens",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    request_body = IssueTokenRequest,
    responses(
        (status = 200, description = "签发成功", body = IssueTokenResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "权限不足或房间不可进入"),
        (status = 404, description = "房间不存在")
    ),
    tag = "rooms"
)]
pub async fn issue_token(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<IssueTokenRequest>,
) -> HandlerResult<IssueTokenResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let mut previous_jti = None;
    let room = if let Some(token) = payload.token.as_deref() {
        let verified = verify_room_token(app_state.clone(), &name, token).await?;
        previous_jti = Some(verified.record.jti.clone());
        verified.room
    } else {
        let repository = SqliteRoomRepository::new(app_state.db_pool.clone());
        let Some(room) = repository.find_by_name(&name).await.map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Database error: {e}"))
        })?
        else {
            return Err(HttpResponse::NotFound().message("Room not found"));
        };

        if let Some(expected_password) = room.password.as_ref()
            && payload.password.as_deref() != Some(expected_password.as_str())
        {
            return Err(HttpResponse::Forbidden().message("Invalid room password"));
        }

        room
    };

    if !room.can_enter() {
        return Err(HttpResponse::Forbidden().message("Room cannot be entered"));
    }

    let (token, claims) = app_state
        .token_service
        .issue(&room)
        .map_err(|e| HttpResponse::Forbidden().message(e.to_string()))?;

    let record = RoomToken::new(claims.room_id, claims.jti.clone(), claims.expires_at());
    let token_repo = SqliteRoomTokenRepository::new(app_state.db_pool.clone());
    token_repo.create(&record).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to persist token: {e}"))
    })?;

    if let Some(jti) = previous_jti {
        token_repo.revoke(&jti).await.map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Failed to revoke old token: {e}"))
        })?;
    }

    Ok(Json(IssueTokenResponse {
        token,
        expires_at: claims.expires_at(),
        claims,
    }))
}

/// 校验房间访问凭证
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/tokens/validate",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    request_body = ValidateTokenRequest,
    responses(
        (status = 200, description = "凭证有效", body = ValidateTokenResponse),
        (status = 401, description = "凭证无效或已撤销"),
        (status = 404, description = "房间不存在")
    ),
    tag = "rooms"
)]
pub async fn validate_token(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<ValidateTokenRequest>,
) -> HandlerResult<ValidateTokenResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let verified = verify_room_token(app_state, &name, &payload.token).await?;

    Ok(Json(ValidateTokenResponse {
        claims: verified.claims,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/permissions",
    params(
        ("name" = String, Path, description = "房间 slug"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = UpdateRoomPermissionRequest,
    responses(
        (status = 200, description = "权限更新成功", body = Room),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效或已撤销"),
        (status = 403, description = "无更新权限"),
        (status = 404, description = "房间不存在")
    ),
    tag = "rooms"
)]
pub async fn update_permissions(
    Path(name): Path<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRoomPermissionRequest>,
) -> HandlerResult<Room> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    let token_perm = verified.claims.as_permission();
    if !token_perm.can_delete() {
        return Err(HttpResponse::Forbidden().message("Permission denied by token"));
    }

    let mut new_permission = RoomPermission::VIEW_ONLY;
    if payload.edit {
        new_permission = new_permission.with_edit();
    }
    if payload.share {
        new_permission = new_permission.with_share();
    }
    if payload.delete {
        new_permission = new_permission.with_delete();
    }

    let repo = SqliteRoomRepository::new(app_state.db_pool.clone());
    let mut room = verified.room;
    let was_shareable = room.permission.can_share();
    room.permission = new_permission;

    if payload.share {
        let desired_slug = room.name.clone();
        if desired_slug != room.slug {
            let exists = repo.exists(&desired_slug).await.map_err(|e| {
                HttpResponse::InternalServerError().message(format!("Database error: {e}"))
            })?;
            if exists {
                return Err(HttpResponse::BadRequest().message("Slug already in use"));
            }
            room.slug = desired_slug;
        }
    } else if was_shareable || room.slug == room.name {
        // 生成私有 slug，避免冲突
        loop {
            let candidate = format!("{}_{}", room.name, Uuid::new_v4());
            let exists = repo.exists(&candidate).await.map_err(|e| {
                HttpResponse::InternalServerError().message(format!("Database error: {e}"))
            })?;
            if !exists {
                room.slug = candidate;
                break;
            }
        }
    }

    let updated_room = repo.update(&room).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to update room: {e}"))
    })?;

    Ok(Json(updated_room))
}

/// 获取房间凭证列表
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}/tokens",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "任一有效房间 token")
    ),
    responses(
        (status = 200, description = "凭证列表", body = [RoomTokenView]),
        (status = 401, description = "凭证无效或已撤销"),
        (status = 404, description = "房间不存在")
    ),
    tag = "rooms"
)]
pub async fn list_tokens(
    Path(name): Path<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<RoomTokenView>> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| HttpResponse::InternalServerError().message("Room id missing"))?;

    let token_repo = SqliteRoomTokenRepository::new(app_state.db_pool.clone());
    let tokens = token_repo.list_by_room(room_id).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to load tokens: {e}"))
    })?;

    Ok(Json(tokens.into_iter().map(RoomTokenView::from).collect()))
}

/// 撤销房间凭证
#[utoipa::path(
    delete,
    path = "/api/v1/rooms/{name}/tokens/{jti}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("jti" = String, Path, description = "要撤销的 token 标识"),
        ("token" = String, Query, description = "任一有效房间 token")
    ),
    responses(
        (status = 200, description = "撤销结果", body = RevokeTokenResponse),
        (status = 401, description = "凭证无效或已撤销"),
        (status = 404, description = "房间不存在")
    ),
    tag = "rooms"
)]
pub async fn revoke_token(
    Path((name, target_jti)): Path<(String, String)>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<RevokeTokenResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let _verified = verify_room_token(app_state.clone(), &name, &query.token).await?;

    let token_repo = SqliteRoomTokenRepository::new(app_state.db_pool.clone());
    let revoked = token_repo.revoke(&target_jti).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to revoke token: {e}"))
    })?;

    Ok(Json(RevokeTokenResponse { revoked }))
}
