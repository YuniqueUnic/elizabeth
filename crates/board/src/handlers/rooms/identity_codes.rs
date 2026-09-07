use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use chrono::{Duration, NaiveDateTime, Utc};

use super::shared::HandlerResult;
use crate::authz::{Authz, Resource, load_role_table};
use crate::dto::rooms::{
    CreateRoomIdentityCodeRequest, CreateRoomIdentityCodeResponse, IssueTokenResponse,
    RedeemRoomIdentityCodeRequest, RoomIdentityCodeView, UpdateRoomIdentityCodeRequest,
    UpdateRoomIdentityCodeResponse,
};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::room::role::{Capability, ROLE_ADMIN};
use crate::models::{Room, RoomIdentityCode, RoomToken};
use crate::repository::{
    IRoomIdentityCodeRepository, IRoomRepository, IRoomTokenRepository, RoomAccessRepository,
    RoomIdentityCodeRepository, RoomRepository, RoomTokenRepository,
};
use crate::services::token::{MAX_IDENTITY_TTL_SECONDS, MIN_IDENTITY_TTL_SECONDS};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/identity-codes",
    params(("name" = String, Path, description = "房间名称")),
    request_body = CreateRoomIdentityCodeRequest,
    responses((status = 200, body = CreateRoomIdentityCodeResponse)),
    tag = "rooms"
)]
pub async fn create_identity_code(
    Path(name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateRoomIdentityCodeRequest>,
) -> HandlerResult<CreateRoomIdentityCodeResponse> {
    let (room, room_id, creator_jti) = require_manager(&app_state, &name, &token).await?;
    let code = validate_identity_code(&payload.code)?;
    let role_key = payload.role.trim();
    ensure_role_exists(&app_state, &room, room_id, role_key).await?;
    let expires_at = identity_code_expiry(&app_state, &room, role_key, payload.expires_in_secs)?;
    let hash = app_state
        .room_password_service()
        .hash(code.clone())
        .await
        .map_err(|e| AppError::internal(format!("Failed to protect identity code: {e}")))?;
    let now = Utc::now().naive_utc();
    let created = RoomIdentityCodeRepository::new(app_state.db_pool.clone())
        .create(&RoomIdentityCode {
            id: None,
            room_id,
            code_hash: hash,
            role_key: role_key.to_owned(),
            expires_at,
            revoked_at: None,
            created_by_jti: Some(creator_jti),
            created_at: now,
            updated_at: now,
        })
        .await
        .map_err(|e| AppError::internal(format!("Failed to create identity code: {e}")))?;
    Ok(Json(CreateRoomIdentityCodeResponse {
        identity_code: code_view(created),
        code,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}/identity-codes",
    params(("name" = String, Path, description = "房间名称")),
    responses((status = 200, body = [RoomIdentityCodeView])),
    tag = "rooms"
)]
pub async fn list_identity_codes(
    Path(name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<RoomIdentityCodeView>> {
    let (_, room_id, _) = require_manager(&app_state, &name, &token).await?;
    let codes = RoomIdentityCodeRepository::new(app_state.db_pool.clone())
        .list_by_room(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list identity codes: {e}")))?;
    Ok(Json(codes.into_iter().map(code_view).collect()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/rooms/{name}/identity-codes/{id}",
    params(("name" = String, Path, description = "房间名称"), ("id" = i64, Path, description = "身份码 ID")),
    request_body = UpdateRoomIdentityCodeRequest,
    responses((status = 200, body = UpdateRoomIdentityCodeResponse)),
    tag = "rooms"
)]
pub async fn update_identity_code(
    Path((name, id)): Path<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRoomIdentityCodeRequest>,
) -> HandlerResult<UpdateRoomIdentityCodeResponse> {
    let (room, room_id, _) = require_manager(&app_state, &name, &token).await?;
    if payload.disable && (payload.code.is_some() || payload.expires_in_secs.is_some()) {
        return Err(AppError::validation(
            "disable cannot be combined with reset or expiry update",
        ));
    }
    if !payload.disable && payload.code.is_none() && payload.expires_in_secs.is_none() {
        return Err(AppError::validation(
            "provide code, expires_in_secs, or disable",
        ));
    }
    let code_repo = RoomIdentityCodeRepository::new(app_state.db_pool.clone());
    let current = code_repo
        .find_by_id(room_id, id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to load identity code: {e}")))?
        .ok_or_else(|| AppError::not_found("Room identity code"))?;
    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    if payload.disable {
        token_repo
            .revoke_active_by_identity_code(room_id, id)
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to revoke identity-code sessions: {e}"))
            })?;
        let revoked = code_repo
            .revoke(room_id, id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to disable identity code: {e}")))?;
        if !revoked {
            return Err(AppError::conflict("Identity code is already disabled"));
        }
        let updated = code_repo
            .find_by_id(room_id, id)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .ok_or_else(|| AppError::not_found("Room identity code"))?;
        return Ok(Json(UpdateRoomIdentityCodeResponse {
            identity_code: code_view(updated),
            code: None,
        }));
    }
    let expires_at = identity_code_expiry(
        &app_state,
        &room,
        &current.role_key,
        payload.expires_in_secs,
    )?;
    if let Some(raw_code) = payload.code {
        let code = validate_identity_code(&raw_code)?;
        let hash = app_state
            .room_password_service()
            .hash(code.clone())
            .await
            .map_err(|e| AppError::internal(format!("Failed to protect identity code: {e}")))?;
        token_repo
            .revoke_active_by_identity_code(room_id, id)
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to revoke identity-code sessions: {e}"))
            })?;
        let updated = code_repo
            .reset(room_id, id, hash, expires_at)
            .await
            .map_err(|e| AppError::internal(format!("Failed to reset identity code: {e}")))?;
        return Ok(Json(UpdateRoomIdentityCodeResponse {
            identity_code: code_view(updated),
            code: Some(code),
        }));
    }
    let updated = code_repo
        .update_expiry(room_id, id, expires_at)
        .await
        .map_err(|e| AppError::internal(format!("Failed to update identity code: {e}")))?;
    Ok(Json(UpdateRoomIdentityCodeResponse {
        identity_code: code_view(updated),
        code: None,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/identity-codes/redeem",
    params(("name" = String, Path, description = "房间名称")),
    request_body = RedeemRoomIdentityCodeRequest,
    responses((status = 200, body = IssueTokenResponse)),
    tag = "rooms"
)]
pub async fn redeem_identity_code(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<RedeemRoomIdentityCodeRequest>,
) -> HandlerResult<IssueTokenResponse> {
    RoomNameValidator::validate_identifier(&name)?;
    let code = validate_identity_code(&payload.code)?;
    let room = RoomRepository::new(app_state.db_pool.clone())
        .find_by_name(&name)
        .await
        .map_err(|e| AppError::internal(format!("Failed to load room: {e}")))?
        .ok_or_else(|| AppError::room_not_found(&name))?;
    if room.is_expired() {
        return Err(AppError::room_expired(name));
    }
    if !room.can_enter() {
        return Err(AppError::authentication("Room cannot be entered"));
    }
    let room_id = room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    let now = Utc::now().naive_utc();
    let code_repo = RoomIdentityCodeRepository::new(app_state.db_pool.clone());
    let codes = code_repo
        .list_by_room(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to load identity codes: {e}")))?;
    let identity_code = find_matching_identity_code(&app_state, code, codes, now).await?;
    ensure_role_exists(&app_state, &room, room_id, &identity_code.role_key).await?;
    let ttl = identity_code.expires_at - now;
    let (token, claims) = app_state
        .token_service()
        .issue_with_ttl(&room, &identity_code.role_key, ttl)
        .map_err(|e| AppError::authentication(format!("Cannot redeem identity code: {e}")))?;
    let identity_code_id = identity_code
        .id
        .ok_or_else(|| AppError::internal("Identity code id missing"))?;
    let record = RoomToken::new(
        claims.room_id,
        claims.jti.clone(),
        &identity_code.role_key,
        claims.expires_at(),
    )
    .with_identity_code_id(identity_code_id);
    let granted = RoomAccessRepository::new(app_state.db_pool.clone())
        .grant_new_session(room_id, &record, None, now)
        .await
        .map_err(|e| AppError::internal(format!("Failed to persist identity-code session: {e}")))?;
    if !granted {
        return Err(AppError::authentication("Room cannot be entered"));
    }
    let role_table = load_role_table(
        &app_state.roles_cache,
        &app_state.db_pool,
        room_id,
        room.roles_version,
    )
    .await?;
    Ok(Json(IssueTokenResponse {
        token,
        expires_at: claims.expires_at(),
        claims,
        capabilities: role_table
            .grants(&identity_code.role_key)
            .unwrap_or_default()
            .to_vec(),
        refresh_token: None,
        refresh_token_expires_at: None,
    }))
}

async fn require_manager(
    app_state: &Arc<AppState>,
    name: &str,
    token: &str,
) -> Result<(Room, i64, String), AppError> {
    RoomNameValidator::validate_identifier(name)?;
    let verified = verify_room_token(app_state.clone(), name, token).await?;
    let room_id = verified
        .room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;
    Authz::for_claims(app_state, &verified.room, &verified.claims)
        .await?
        .require(Capability::RoomRolesManage, &Resource::Room { room_id })?;
    Ok((verified.room, room_id, verified.record.jti))
}

async fn ensure_role_exists(
    app_state: &AppState,
    room: &Room,
    room_id: i64,
    role_key: &str,
) -> Result<(), AppError> {
    if role_key.is_empty() {
        return Err(AppError::validation("role must not be empty"));
    }
    let roles = load_role_table(
        &app_state.roles_cache,
        &app_state.db_pool,
        room_id,
        room.roles_version,
    )
    .await?;
    if !roles.contains_key(role_key) {
        return Err(AppError::validation(
            "Requested role does not exist in this room",
        ));
    }
    Ok(())
}

fn identity_code_expiry(
    app_state: &AppState,
    room: &Room,
    role_key: &str,
    requested_secs: Option<i64>,
) -> Result<NaiveDateTime, AppError> {
    if role_key == ROLE_ADMIN {
        if requested_secs.is_some() {
            return Err(AppError::validation(
                "admin identity code expiry follows the room lifetime",
            ));
        }
        return room
            .expire_at
            .ok_or_else(|| AppError::internal("Room has no expiry for admin identity code"));
    }
    let seconds =
        requested_secs.unwrap_or_else(|| app_state.token_service().get_ttl().num_seconds());
    if !(MIN_IDENTITY_TTL_SECONDS..=MAX_IDENTITY_TTL_SECONDS).contains(&seconds) {
        return Err(AppError::validation(format!(
            "expires_in_secs must be between {MIN_IDENTITY_TTL_SECONDS} and {MAX_IDENTITY_TTL_SECONDS} seconds"
        )));
    }
    let expiry = Utc::now().naive_utc() + Duration::seconds(seconds);
    Ok(room
        .expire_at
        .map_or(expiry, |room_expiry| expiry.min(room_expiry)))
}

async fn find_matching_identity_code(
    app_state: &AppState,
    code: String,
    codes: Vec<RoomIdentityCode>,
    now: NaiveDateTime,
) -> Result<RoomIdentityCode, AppError> {
    for candidate in codes
        .into_iter()
        .filter(|candidate| candidate.is_active_at(now))
    {
        let valid = app_state
            .room_password_service()
            .verify(code.clone(), candidate.code_hash.clone())
            .await
            .map_err(|e| AppError::internal(format!("Failed to verify identity code: {e}")))?;
        if valid {
            return Ok(candidate);
        }
    }
    Err(AppError::authentication("Invalid identity code"))
}

pub(crate) fn validate_identity_code(raw: &str) -> Result<String, AppError> {
    let code = raw.trim();
    let valid_chars = code
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
    let jwt_shape = code.split('.').count() == 3 && code.split('.').all(|part| !part.is_empty());
    if !(6..=128).contains(&code.len()) || !code.is_ascii() || !valid_chars || jwt_shape {
        return Err(AppError::validation(
            "identity code must be 6-128 ASCII characters using letters, digits, _, -, or . and must not be a JWT",
        ));
    }
    Ok(code.to_owned())
}

fn code_view(code: RoomIdentityCode) -> RoomIdentityCodeView {
    RoomIdentityCodeView {
        id: code.id.unwrap_or_default(),
        role: code.role_key,
        expires_at: code.expires_at,
        revoked_at: code.revoked_at,
        created_at: code.created_at,
        updated_at: code.updated_at,
    }
}
