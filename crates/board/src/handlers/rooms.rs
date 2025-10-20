use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum_responses::http::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use super::verify_room_token;
use crate::models::{Room, RoomToken};
use crate::repository::{
    IRoomRepository, IRoomTokenRepository, SqliteRoomRepository, SqliteRoomTokenRepository,
};
use crate::services::RoomTokenClaims;
use crate::state::AppState;

type HandlerResult<T> = Result<Json<T>, HttpResponse>;

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

    let room = Room::new(name.clone(), params.password);
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
            // 如果房间不存在，创建一个新的
            let new_room = Room::new(name, None);
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
        ("name" = String, Path, description = "房间名称")
    ),
    responses(
        (status = 200, description = "房间删除成功"),
        (status = 404, description = "房间不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn delete(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if name.is_empty() {
        return HttpResponse::BadRequest()
            .message("Invalid room name")
            .into_response();
    }

    let repository = SqliteRoomRepository::new(app_state.db_pool.clone());

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

    let (room, _existing_claims) = if let Some(token) = payload.token.as_deref() {
        let verified = verify_room_token(app_state.clone(), &name, token).await?;
        (verified.room, Some(verified.claims))
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

        (room, None)
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
