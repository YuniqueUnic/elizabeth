use std::sync::Arc;

use crate::handlers::*;
use crate::state::AppState;
use axum::routing::{delete as axum_delete, get as axum_get, post as axum_post};

// API 路由器
pub fn api_router(app_state: Arc<AppState>) -> utoipa_axum::router::OpenApiRouter {
    utoipa_axum::router::OpenApiRouter::new()
        .route("/api/v1/rooms/{name}", axum_post(create))
        .route("/api/v1/rooms/{name}", axum_get(find))
        .route("/api/v1/rooms/{name}", axum_delete(delete))
        .route(
            "/api/v1/rooms/{name}/permissions",
            axum_post(update_permissions),
        )
        .route("/api/v1/rooms/{name}/tokens", axum_post(issue_token))
        .route("/api/v1/rooms/{name}/tokens", axum_get(list_tokens))
        .route(
            "/api/v1/rooms/{name}/tokens/validate",
            axum_post(validate_token),
        )
        .route(
            "/api/v1/rooms/{name}/tokens/{jti}",
            axum_delete(revoke_token),
        )
        .route("/api/v1/rooms/{name}/contents", axum_get(list_contents))
        .route(
            "/api/v1/rooms/{name}/contents/prepare",
            axum_post(prepare_upload),
        )
        .route("/api/v1/rooms/{name}/contents", axum_post(upload_contents))
        .route(
            "/api/v1/rooms/{name}/contents",
            axum_delete(delete_contents),
        )
        .route(
            "/api/v1/rooms/{name}/contents/{content_id}",
            axum_get(download_content),
        )
        .with_state(app_state)
}
