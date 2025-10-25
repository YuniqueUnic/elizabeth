use std::{path::PathBuf, sync::Arc};

use chrono::Duration;

use crate::db::DbPool;
use crate::services::{RoomTokenService, refresh_token_service::RefreshTokenService};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<DbPool>,
    pub token_service: RoomTokenService,
    pub refresh_token_service: RefreshTokenService,
    pub storage_root: Arc<PathBuf>,
    pub upload_reservation_ttl: Duration,
    pub room_max_size: i64,
    pub room_max_times_entered: i64,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db_pool: Arc<DbPool>,
        storage_root: impl Into<PathBuf>,
        upload_reservation_ttl: Duration,
        room_max_size: i64,
        room_max_times_entered: i64,
        token_service: RoomTokenService,
        refresh_token_service: RefreshTokenService,
    ) -> Self {
        Self {
            db_pool,
            token_service,
            refresh_token_service,
            storage_root: Arc::new(storage_root.into()),
            upload_reservation_ttl,
            room_max_size,
            room_max_times_entered,
        }
    }
}
