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
use crate::validation::{RoomNameValidator, TokenValidator};

/// 提取统一的房间身份码：Bearer、X-API-Key 或 ?token= 均承载同一个已签发的房间 JWT。
/// Authorization 优先，其次是 API key，查询参数仅用于浏览器媒体链接。
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

            // 2. 机器调用的快捷身份码：其值仍是同一个房间 JWT，不创建第二套授权模型。
            if let Some(api_key) = parts.headers.get("x-api-key")
                && let Ok(token) = api_key.to_str()
                && !token.trim().is_empty()
            {
                return Ok(AuthToken(token.trim().to_string()));
            }

            // 3. 回退到 ?token= 查询参数
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

/// 纯「身份 + Room Gate」校验：验签、jti 存活、房间匹配/未过期/开放。
/// 能力判定一律交给 `authz::Authz`（实时角色矩阵），这里不做任何能力检查。
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
        return Err(AppError::room_expired(room_name));
    }
    if room.status() != RoomStatus::Open {
        return Err(AppError::authentication("Room cannot be entered"));
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

    let claims = resolve_role_claims(claims, &record, &room);
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
        return Err(AppError::room_expired(room.slug.clone()));
    }
    if room.status() != RoomStatus::Open {
        return Err(AppError::authentication("Room cannot be entered"));
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

    let claims = resolve_role_claims(claims, &record, &room);
    Ok(VerifiedRoomToken {
        room,
        claims,
        record,
    })
}

/// 角色 key 兜底：DB 记录 > claims 快照 > 房间默认角色。
/// 旧 JWT（无 role claim）在 access TTL 内经此回填后自然淘汰，不构成长期兼容层。
fn resolve_role_claims(
    mut claims: RoomTokenClaims,
    record: &RoomToken,
    room: &Room,
) -> RoomTokenClaims {
    if let Some(role_key) = record.role_key.clone().filter(|role| !role.is_empty()) {
        claims.role = role_key;
    } else if claims.role.is_empty() {
        claims.role = room.default_role_key.clone();
    }
    claims
}
