use std::sync::Arc;

use crate::db::DbPool;
use crate::services::RoomTokenService;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<DbPool>,
    pub jwt_secret: Arc<String>,
    pub token_service: RoomTokenService,
}

impl AppState {
    pub fn new(db_pool: Arc<DbPool>, jwt_secret: impl Into<String>) -> Self {
        let secret = Arc::new(jwt_secret.into());
        let token_service = RoomTokenService::new(secret.clone());
        Self {
            db_pool,
            jwt_secret: secret,
            token_service,
        }
    }
}
