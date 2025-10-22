use std::sync::Arc;

use crate::handlers::{
    cleanup_expired_tokens, refresh_token::refresh_token as refresh_auth_token,
    refresh_token::revoke_token as revoke_auth_token,
};
use crate::state::AppState;
use axum::routing::{delete as axum_delete, post as axum_post};
use utoipa_axum::router::OpenApiRouter;

/// 认证相关的 API 路由
pub fn auth_router(app_state: Arc<AppState>) -> OpenApiRouter {
    utoipa_axum::router::OpenApiRouter::new()
        .route("/api/v1/auth/refresh", axum_post(refresh_auth_token))
        .route("/api/v1/auth/logout", axum_post(revoke_auth_token))
        .route("/api/v1/auth/cleanup", axum_delete(cleanup_expired_tokens))
        .with_state(app_state)
}
