use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::state::AppState;

pub fn api_router(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(crate::handlers::admin::list_full_unbounded_rooms))
        .routes(routes!(crate::handlers::admin::run_room_gc))
        .with_state(app_state)
}
