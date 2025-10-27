use std::sync::Arc;

use crate::state::AppState;
use utoipa_axum::{router::OpenApiRouter, routes};

/// 认证相关的 API 路由
pub fn auth_router(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(crate::handlers::refresh_token::refresh_token))
        .routes(routes!(crate::handlers::refresh_token::revoke_token))
        .routes(routes!(
            crate::handlers::refresh_token::cleanup_expired_tokens
        ))
        .with_state(app_state)
}
