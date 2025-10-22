use std::sync::Arc;

use axum_responses::http::HttpResponse;

use crate::models::{Room, RoomToken};
use crate::repository::{
    IRoomRepository, IRoomTokenRepository, SqliteRoomRepository, SqliteRoomTokenRepository,
};
use crate::services::RoomTokenClaims;
use crate::state::AppState;

pub struct VerifiedRoomToken {
    pub room: Room,
    pub claims: RoomTokenClaims,
    pub record: RoomToken,
}

pub async fn verify_room_token(
    app_state: Arc<AppState>,
    room_name: &str,
    token_str: &str,
) -> Result<VerifiedRoomToken, HttpResponse> {
    let claims = app_state
        .token_service
        .decode(token_str)
        .map_err(|_| HttpResponse::Unauthorized().message("Token is invalid or expired"))?;

    if claims.room_name != room_name {
        return Err(HttpResponse::Unauthorized().message("Token not issued for this room"));
    }

    let room_repo = SqliteRoomRepository::new(app_state.db_pool.clone());
    let room = room_repo
        .find_by_name(room_name)
        .await
        .map_err(|e| HttpResponse::InternalServerError().message(format!("Database error: {e}")))?
        .ok_or_else(|| HttpResponse::NotFound().message("Room not found"))?;

    if room.id != Some(claims.room_id) {
        return Err(HttpResponse::Unauthorized().message("Token room mismatch"));
    }
    if room.is_expired() {
        return Err(HttpResponse::Unauthorized().message("Room expired"));
    }
    if !room.can_enter() {
        return Err(HttpResponse::Unauthorized().message("Room cannot be entered"));
    }

    let token_repo = SqliteRoomTokenRepository::new(app_state.db_pool.clone());
    let record = token_repo
        .find_by_jti(&claims.jti)
        .await
        .map_err(|e| HttpResponse::InternalServerError().message(format!("Database error: {e}")))?
        .ok_or_else(|| HttpResponse::Unauthorized().message("Token revoked or not found"))?;

    if !record.is_active() {
        return Err(HttpResponse::Unauthorized().message("Token revoked or expired"));
    }

    Ok(VerifiedRoomToken {
        room,
        claims,
        record,
    })
}
