use std::sync::Arc;

use crate::db::DbPool;
use crate::handlers::*;

// API 路由器
pub fn api_router(db_pool: Arc<DbPool>) -> utoipa_axum::router::OpenApiRouter {
    utoipa_axum::router::OpenApiRouter::new()
        .routes(utoipa_axum::routes!(create, find, delete))
        .with_state(db_pool)
}
