use std::sync::Arc;

use crate::handlers::{
    content::{delete_contents, download_content, list_contents, prepare_upload, upload_contents},
    rooms::{
        create, delete, find, issue_token, list_tokens, revoke_token as revoke_room_token,
        update_permissions, validate_token,
    },
};
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
            axum_delete(revoke_room_token),
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
        .route(
            "/api/v1/rooms/{name}/uploads/chunks/prepare",
            axum_post(crate::handlers::prepare_chunked_upload),
        )
        .route(
            "/api/v1/rooms/{name}/uploads/chunks",
            axum_post(crate::handlers::upload_chunk),
        )
        .route(
            "/api/v1/rooms/{name}/uploads/chunks/status",
            axum_get(crate::handlers::get_upload_status),
        )
        .route(
            "/api/v1/rooms/{name}/uploads/chunks/complete",
            axum_post(crate::handlers::complete_file_merge),
        )
        .with_state(app_state)
}
