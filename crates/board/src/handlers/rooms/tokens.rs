use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;

use super::shared::HandlerResult;
use crate::authz::{Authz, Resource, load_role_table};
use crate::dto::rooms::{
    IssueTokenRequest, IssueTokenResponse, RevokeTokenResponse, RoomTokenView,
    ValidateTokenRequest, ValidateTokenResponse, VerifyRoomPasswordRequest,
    VerifyRoomPasswordResponse,
};
use crate::errors::AppError;
use crate::handlers::admin::validate_admin_credential;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::room::role::Capability;
use crate::models::{Room, RoomStatus, RoomToken};
use crate::repository::{
    IRoomRepository, IRoomTokenRepository, RoomAccessRepository, RoomRepository,
    RoomTokenRepository,
};
use crate::state::AppState;
use crate::validation::{RoomNameValidator, TokenValidator};

struct TokenIssueContext {
    room: Room,
    previous_jti: Option<String>,
    /// 续签场景下的请求者（匿名进房为 None）
    requester: Option<crate::handlers::VerifiedRoomToken>,
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
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<IssueTokenRequest>,
) -> HandlerResult<IssueTokenResponse> {
    RoomNameValidator::validate_identifier(&name)?;
    let admin_bootstrap = validate_admin_credential(
        headers
            .get("X-Elizabeth-Admin-Token")
            .and_then(|value| value.to_str().ok()),
    )
    .is_ok();

    let TokenIssueContext {
        mut room,
        previous_jti,
        requester,
    } = resolve_token_issue_room(&app_state, &name, &payload).await?;
    let should_increment_view_count = previous_jti.is_none();
    ensure_token_issue_allowed(&room, should_increment_view_count)?;

    // 角色解析：请求角色必须存在于房间矩阵；非默认角色需要 RoomRolesManage 能力。
    let room_id = room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    let role_table = load_role_table(
        &app_state.roles_cache,
        &app_state.db_pool,
        room_id,
        room.roles_version,
    )
    .await?;
    let requested_role = payload
        .role
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty());
    let role_key = match requested_role {
        Some(requested) => {
            if !role_table.contains_key(requested) {
                return Err(AppError::validation(
                    "Requested role does not exist in this room",
                ));
            }
            if requested != room.default_role_key {
                if let Some(verified) = requester.as_ref() {
                    let authz = Authz::for_claims(&app_state, &room, &verified.claims).await?;
                    authz.require(Capability::RoomRolesManage, &Resource::Room { room_id })?;
                } else if !admin_bootstrap {
                    return Err(AppError::permission_denied(
                        "Joining with a non-default role requires room.roles.manage or admin credential",
                    ));
                }
            }
            requested.to_string()
        }
        None => requester
            .as_ref()
            .and_then(|verified| verified.record.role_key.clone())
            .or_else(|| {
                requester
                    .as_ref()
                    .map(|verified| verified.claims.role.clone())
            })
            .unwrap_or_else(|| room.default_role_key.clone()),
    };

    let previous_jti = requester.as_ref().and_then(|verified| {
        let current_role = verified
            .record
            .role_key
            .as_deref()
            .unwrap_or(verified.claims.role.as_str());
        (current_role == role_key).then(|| verified.record.jti.clone())
    });

    let (token, claims) = app_state
        .token_service()
        .issue(&room, &role_key)
        .map_err(|e| AppError::authentication(e.to_string()))?;
    let record = RoomToken::new(
        claims.room_id,
        claims.jti.clone(),
        &role_key,
        claims.expires_at(),
    );
    let prepared_refresh = if payload.with_refresh_token {
        Some(
            app_state
                .refresh_token_service()
                .prepare_refresh_token(&room, &role_key, claims.jti.clone())
                .map_err(|e| AppError::internal(format!("Failed to issue refresh token: {e}")))?,
        )
    } else {
        None
    };
    let access_repo = RoomAccessRepository::new(app_state.db_pool.clone());
    let granted = if let Some(previous_jti) = previous_jti.as_deref() {
        access_repo
            .rotate_access_token(
                claims.room_id,
                previous_jti,
                &record,
                prepared_refresh.as_ref().map(|prepared| &prepared.record),
                chrono::Utc::now().naive_utc(),
            )
            .await
    } else {
        access_repo
            .grant_new_session(
                claims.room_id,
                &record,
                prepared_refresh.as_ref().map(|prepared| &prepared.record),
                chrono::Utc::now().naive_utc(),
            )
            .await
    }
    .map_err(|e| {
        if e.to_string().contains("maximum active editor identities") {
            AppError::conflict("Maximum active editor identities reached")
        } else {
            AppError::internal(format!("Failed to persist room access grant: {e}"))
        }
    })?;

    if !granted {
        return Err(AppError::authentication("Room cannot be entered"));
    }
    if should_increment_view_count {
        room.current_times_entered += 1;
    }

    let (refresh_token, refresh_expires_at) = prepared_refresh
        .map(|prepared| (Some(prepared.signed_token), Some(prepared.expires_at)))
        .unwrap_or((None, None));
    broadcast_user_joined(app_state, name, claims.jti.clone());

    Ok(Json(IssueTokenResponse {
        token,
        expires_at: claims.expires_at(),
        claims,
        capabilities: role_table.grants(&role_key).unwrap_or_default().to_vec(),
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
        (status = 403, description = "缺少 room.roles.manage 能力"),
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
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    authz.require(Capability::RoomRolesManage, &Resource::Room { room_id })?;

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
        (status = 403, description = "缺少 room.roles.manage 能力"),
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

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    authz.require(Capability::RoomRolesManage, &Resource::Room { room_id })?;

    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    let target = token_repo
        .find_by_jti(&target_jti)
        .await
        .map_err(|e| AppError::internal(format!("Failed to load token: {e}")))?
        .ok_or_else(|| AppError::not_found("Room token"))?;
    if target.room_id != room_id {
        return Err(AppError::not_found("Room token"));
    }
    let revoked = token_repo
        .revoke(&target_jti)
        .await
        .map_err(|e| AppError::internal(format!("Failed to revoke token: {e}")))?;
    if revoked {
        app_state
            .connection_manager
            .disconnect_room(&verified.room.slug, "Session revoked")
            .await;
    }

    Ok(Json(RevokeTokenResponse { revoked }))
}

async fn resolve_token_issue_room(
    app_state: &Arc<AppState>,
    name: &str,
    payload: &IssueTokenRequest,
) -> Result<TokenIssueContext, AppError> {
    if let Some(token) = payload.token.as_deref() {
        TokenValidator::validate_token_format(token)?;
        let verified = verify_room_token(app_state.clone(), name, token).await?;
        Ok(TokenIssueContext {
            previous_jti: Some(verified.record.jti.clone()),
            room: verified.room.clone(),
            requester: Some(verified),
        })
    } else {
        let repository = RoomRepository::new(app_state.db_pool.clone());
        let room = repository
            .find_by_name(name)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| AppError::room_not_found(name))?;
        if room.is_expired() {
            return Err(AppError::room_expired(name));
        }
        validate_room_password(app_state, &room, payload).await?;
        Ok(TokenIssueContext {
            room,
            previous_jti: None,
            requester: None,
        })
    }
}

async fn validate_room_password(
    app_state: &AppState,
    room: &Room,
    payload: &IssueTokenRequest,
) -> Result<(), AppError> {
    if let Some(encoded_hash) = room.password.as_ref() {
        let password = payload
            .password
            .clone()
            .ok_or_else(|| AppError::authentication("Invalid room password"))?;
        let valid = app_state
            .room_password_service()
            .verify(password, encoded_hash.clone())
            .await
            .map_err(|e| AppError::internal(format!("Failed to verify room password: {e}")))?;
        if !valid {
            return Err(AppError::authentication("Invalid room password"));
        }
    }
    Ok(())
}

fn ensure_token_issue_allowed(room: &Room, increment_view_count: bool) -> Result<(), AppError> {
    if room.is_expired() {
        return Err(AppError::room_expired(room.slug.clone()));
    }
    if increment_view_count {
        if !room.can_enter() {
            return Err(AppError::authentication("Room cannot be entered"));
        }
    } else if room.status() != RoomStatus::Open {
        return Err(AppError::authentication("Room cannot be entered"));
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/password/verify",
    params(("name" = String, Path, description = "房间名称")),
    request_body = VerifyRoomPasswordRequest,
    responses(
        (status = 200, description = "密码正确", body = VerifyRoomPasswordResponse),
        (status = 401, description = "密码错误"),
        (status = 404, description = "房间不存在"),
        (status = 410, description = "房间已过期")
    ),
    tag = "rooms"
)]
pub async fn verify_password(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<VerifyRoomPasswordRequest>,
) -> HandlerResult<VerifyRoomPasswordResponse> {
    RoomNameValidator::validate_identifier(&name)?;
    let room = RoomRepository::new(app_state.db_pool.clone())
        .find_by_name(&name)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::room_not_found(&name))?;
    if room.is_expired() {
        return Err(AppError::room_expired(name));
    }

    let request = IssueTokenRequest {
        password: Some(payload.password),
        token: None,
        with_refresh_token: false,
        role: None,
    };
    validate_room_password(&app_state, &room, &request).await?;
    Ok(Json(VerifyRoomPasswordResponse { valid: true }))
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
