use std::sync::Arc;

use crate::errors::{AppError, AppResult};
use crate::models::{Room, RoomToken};
use crate::repository::{
    IRoomRepository, IRoomTokenRepository, SqliteRoomRepository, SqliteRoomTokenRepository,
};
use crate::services::RoomTokenClaims;
use crate::state::AppState;
use crate::validation::RoomNameValidator;
use crate::validation::TokenValidator;

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
    RoomNameValidator::validate(room_name)?;

    // 验证令牌格式
    TokenValidator::validate_token_format(token_str)?;

    // 解码令牌
    let claims = app_state
        .token_service
        .decode(token_str)
        .map_err(|e| AppError::token(format!("Token is invalid or expired: {}", e)))?;

    // 验证令牌是否为该房间签发
    if claims.room_name != room_name {
        return Err(AppError::authentication("Token not issued for this room"));
    }

    // 查找房间
    let room_repo = SqliteRoomRepository::new(app_state.db_pool.clone());
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
    if !room.can_enter() {
        return Err(AppError::authentication("Room cannot be entered"));
    }

    // 查找令牌记录
    let token_repo = SqliteRoomTokenRepository::new(app_state.db_pool.clone());
    let record = token_repo
        .find_by_jti(&claims.jti)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::authentication("Token revoked or not found"))?;

    // 验证令牌状态
    if !record.is_active() {
        return Err(AppError::authentication("Token revoked or expired"));
    }

    Ok(VerifiedRoomToken {
        room,
        claims,
        record,
    })
}
