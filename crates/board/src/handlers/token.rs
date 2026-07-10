use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::errors::{AppError, AppResult};
use crate::models::{Room, RoomStatus, RoomToken};
use crate::repository::{
    IRoomRepository, IRoomTokenRepository, RoomRepository, RoomTokenRepository,
};
use crate::services::RoomTokenClaims;
use crate::state::AppState;
use crate::validation::RoomNameValidator;
use crate::validation::TokenValidator;

/// 从 Authorization header 或 ?token= 查询参数中提取 JWT token。
/// 优先使用 Authorization header，回退到查询参数。
pub struct AuthToken(pub String);

impl<S: Send + Sync> FromRequestParts<S> for AuthToken {
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = (|| {
            // 1. 优先从 Authorization header 提取
            if let Some(auth_header) = parts.headers.get("authorization")
                && let Ok(header_str) = auth_header.to_str()
                && let Ok(token) = TokenValidator::extract_from_auth_header(header_str)
            {
                return Ok(AuthToken(token));
            }

            // 2. 回退到 ?token= 查询参数
            if let Some(query) = parts.uri.query() {
                for pair in query.split('&') {
                    if let Some((key, value)) = pair.split_once('=')
                        && key == "token"
                        && !value.is_empty()
                    {
                        return Ok(AuthToken(value.to_string()));
                    }
                }
            }

            Err(AppError::authentication("Missing authentication token"))
        })();

        std::future::ready(result)
    }
}

pub struct VerifiedRoomToken {
    pub room: Room,
    pub claims: RoomTokenClaims,
    pub record: RoomToken,
}

pub async fn verify_room_token(
    app_state: Arc<AppState>,
    room_name: &str,
    token_str: &str,
) -> AppResult<VerifiedRoomToken> {
    // 验证房间名称
    RoomNameValidator::validate_identifier(room_name)?;

    // 验证令牌格式
    TokenValidator::validate_token_format(token_str)?;

    // 解码令牌
    let claims = app_state
        .token_service()
        .decode(token_str)
        .map_err(|e| AppError::token(format!("Token is invalid or expired: {}", e)))?;

    // 验证令牌是否为该房间签发
    if claims.room_name != room_name {
        return Err(AppError::authentication("Token not issued for this room"));
    }

    // 查找房间
    let room_repo = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repo
        .find_by_name(room_name)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::room_not_found(room_name))?;

    // 验证房间状态
    if room.id != Some(claims.room_id) {
        return Err(AppError::authentication("Token room mismatch"));
    }
    if room.is_expired() {
        return Err(AppError::authentication("Room expired"));
    }
    if room.status() != RoomStatus::Open {
        return Err(AppError::authentication("Room cannot be entered"));
    }
    if !room.permission.can_view() || !claims.as_permission().can_view() {
        return Err(AppError::permission_denied("Room view permission denied"));
    }

    // 查找令牌记录
    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    let record = token_repo
        .find_by_jti(&claims.jti)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::authentication("Token revoked or not found"))?;

    // 验证令牌状态
    if !record.is_active() {
        return Err(AppError::authentication("Token revoked or expired"));
    }
    if record.room_id != claims.room_id {
        return Err(AppError::authentication("Token record room mismatch"));
    }

    Ok(VerifiedRoomToken {
        room,
        claims,
        record,
    })
}

pub async fn verify_room_token_by_id(
    app_state: Arc<AppState>,
    room_id: i64,
    token_str: &str,
) -> AppResult<VerifiedRoomToken> {
    // 验证令牌格式
    TokenValidator::validate_token_format(token_str)?;

    // 解码令牌
    let claims = app_state
        .token_service()
        .decode(token_str)
        .map_err(|e| AppError::token(format!("Token is invalid or expired: {}", e)))?;

    // 验证令牌是否为该房间 ID 签发
    if claims.room_id != room_id {
        return Err(AppError::authentication("Token room ID mismatch"));
    }

    // 查找房间
    let room_repo = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repo
        .find_by_id(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::room_not_found(format!("ID {room_id}")))?;

    // 验证房间状态
    if room.id != Some(claims.room_id) {
        return Err(AppError::authentication("Token room ID mismatch"));
    }
    if room.is_expired() {
        return Err(AppError::authentication("Room expired"));
    }
    if room.status() != RoomStatus::Open {
        return Err(AppError::authentication("Room cannot be entered"));
    }
    if !room.permission.can_view() || !claims.as_permission().can_view() {
        return Err(AppError::permission_denied("Room view permission denied"));
    }

    // 查找令牌记录
    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    let record = token_repo
        .find_by_jti(&claims.jti)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::authentication("Token revoked or not found"))?;

    // 验证令牌状态
    if !record.is_active() {
        return Err(AppError::authentication("Token revoked or expired"));
    }
    if record.room_id != claims.room_id {
        return Err(AppError::authentication("Token record room mismatch"));
    }

    Ok(VerifiedRoomToken {
        room,
        claims,
        record,
    })
}
