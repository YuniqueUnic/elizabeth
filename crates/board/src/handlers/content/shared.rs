use axum::Json;

use crate::errors::AppError;
use crate::services::RoomTokenClaims;

pub(crate) type HandlerResult<T> = Result<Json<T>, AppError>;

pub(crate) fn room_id_or_error(claims: &RoomTokenClaims) -> Result<i64, AppError> {
    if claims.room_id <= 0 {
        return Err(AppError::internal("Room id missing in claims"));
    }
    Ok(claims.room_id)
}
