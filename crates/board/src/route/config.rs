use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::state::AppState;

pub fn api_router(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(crate::handlers::config::get_public_config))
        .with_state(app_state)
}
