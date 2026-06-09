use axum::Json;

use crate::errors::AppError;
use crate::models::Room;
use crate::state::AppState;
use crate::websocket::types::RoomInfo;

pub(crate) type HandlerResult<T> = Result<Json<T>, AppError>;

pub(crate) fn apply_room_defaults(room: &mut Room, app_state: &AppState) {
    room.max_size = app_state.room_max_size();
    room.max_times_entered = app_state.room_max_times_entered();
}

pub(crate) fn room_info_from_room(room: &Room) -> RoomInfo {
    RoomInfo {
        id: room.id.unwrap_or(0),
        name: room.name.clone(),
        slug: room.slug.clone(),
        max_size: room.max_size,
        current_size: room.current_size,
        max_times_entered: room.max_times_entered,
        current_times_entered: room.current_times_entered,
    }
}
