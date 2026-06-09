use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};

use super::shared::HandlerResult;
use crate::dto::rooms::{
    IssueTokenRequest, IssueTokenResponse, RevokeTokenResponse, RoomTokenView,
    ValidateTokenRequest, ValidateTokenResponse,
};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::{Room, RoomStatus, RoomToken};
use crate::repository::{
    IRoomRepository, IRoomTokenRepository, RoomRepository, RoomTokenRepository,
};
use crate::state::AppState;
use crate::validation::{RoomNameValidator, TokenValidator};

struct TokenIssueRoom {
    room: Room,
    previous_jti: Option<String>,
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
    RoomNameValidator::validate_identifier(&name)?;

    let TokenIssueRoom {
        mut room,
        previous_jti,
    } = resolve_token_issue_room(&app_state, &name, &payload).await?;
    let should_increment_view_count = previous_jti.is_none();
    ensure_token_issue_allowed(&room, should_increment_view_count)?;

    let repository = RoomRepository::new(app_state.db_pool.clone());
    if should_increment_view_count {
        increment_room_view_count(&repository, &mut room).await?;
    } else {
        logrs::debug!(
            "Room {} token refresh, view count not incremented (current: {}/{})",
            room.slug,
            room.current_times_entered,
            room.max_times_entered
        );
    }

    let (token, claims) = app_state
        .token_service()
        .issue(&room)
        .map_err(|e| AppError::authentication(e.to_string()))?;
    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    persist_issued_token(&token_repo, &claims, previous_jti).await?;

    let (refresh_token, refresh_expires_at) =
        issue_refresh_token_if_requested(&app_state, &room, payload.with_refresh_token).await?;
    broadcast_user_joined(app_state, name, claims.jti.clone());

    Ok(Json(IssueTokenResponse {
        token,
        expires_at: claims.expires_at(),
        claims,
        refresh_token,
        refresh_token_expires_at: refresh_expires_at,
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
    RoomNameValidator::validate_identifier(&name)?;

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
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<RoomTokenView>> {
    RoomNameValidator::validate_identifier(&name)?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;

    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    let tokens = token_repo
        .list_by_room(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to load tokens: {e}")))?;

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
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<RevokeTokenResponse> {
    RoomNameValidator::validate_identifier(&name)?;

    let _verified = verify_room_token(app_state.clone(), &name, &token).await?;

    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    let revoked = token_repo
        .revoke(&target_jti)
        .await
        .map_err(|e| AppError::internal(format!("Failed to revoke token: {e}")))?;

    Ok(Json(RevokeTokenResponse { revoked }))
}

async fn resolve_token_issue_room(
    app_state: &Arc<AppState>,
    name: &str,
    payload: &IssueTokenRequest,
) -> Result<TokenIssueRoom, AppError> {
    if let Some(token) = payload.token.as_deref() {
        TokenValidator::validate_token_format(token)?;
        let verified = verify_room_token(app_state.clone(), name, token).await?;
        Ok(TokenIssueRoom {
            previous_jti: Some(verified.record.jti.clone()),
            room: verified.room,
        })
    } else {
        let repository = RoomRepository::new(app_state.db_pool.clone());
        let room = repository
            .find_by_name(name)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| AppError::room_not_found(name))?;
        validate_room_password(&room, payload)?;
        Ok(TokenIssueRoom {
            room,
            previous_jti: None,
        })
    }
}

fn validate_room_password(room: &Room, payload: &IssueTokenRequest) -> Result<(), AppError> {
    if let Some(expected_password) = room.password.as_ref()
        && payload.password.as_deref() != Some(expected_password.as_str())
    {
        return Err(AppError::authentication("Invalid room password"));
    }
    Ok(())
}

fn ensure_token_issue_allowed(room: &Room, increment_view_count: bool) -> Result<(), AppError> {
    if increment_view_count {
        if !room.can_enter() {
            return Err(AppError::authentication("Room cannot be entered"));
        }
    } else if room.is_expired() || room.status() == RoomStatus::Close {
        return Err(AppError::authentication("Room cannot be entered"));
    }
    Ok(())
}

async fn increment_room_view_count(
    repository: &RoomRepository,
    room: &mut Room,
) -> Result<(), AppError> {
    room.current_times_entered += 1;
    logrs::info!(
        "Room {} view count incremented to {}/{}",
        room.slug,
        room.current_times_entered,
        room.max_times_entered
    );
    repository
        .update(room)
        .await
        .map_err(|e| AppError::internal(format!("Failed to update room view count: {}", e)))?;
    Ok(())
}

async fn persist_issued_token(
    token_repo: &RoomTokenRepository,
    claims: &crate::services::RoomTokenClaims,
    previous_jti: Option<String>,
) -> Result<(), AppError> {
    let record = RoomToken::new(claims.room_id, claims.jti.clone(), claims.expires_at());
    token_repo
        .create(&record)
        .await
        .map_err(|e| AppError::internal(format!("Failed to persist token: {}", e)))?;

    if let Some(jti) = previous_jti {
        token_repo
            .revoke(&jti)
            .await
            .map_err(|e| AppError::internal(format!("Failed to revoke old token: {}", e)))?;
    }

    Ok(())
}

async fn issue_refresh_token_if_requested(
    app_state: &Arc<AppState>,
    room: &Room,
    with_refresh_token: bool,
) -> Result<(Option<String>, Option<chrono::NaiveDateTime>), AppError> {
    if !with_refresh_token {
        return Ok((None, None));
    }

    let refresh_response = app_state
        .refresh_token_service()
        .issue_token_pair(room)
        .await
        .map_err(|e| AppError::internal(format!("Failed to issue refresh token: {}", e)))?;

    Ok((
        Some(refresh_response.refresh_token),
        Some(refresh_response.refresh_token_expires_at),
    ))
}

fn broadcast_user_joined(app_state: Arc<AppState>, room_name: String, user_id: String) {
    let broadcaster = app_state.broadcaster.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_user_joined(&room_name, &user_id)
            .await
        {
            log::warn!("Failed to broadcast user joined event: {}", e);
        }
    });
}
