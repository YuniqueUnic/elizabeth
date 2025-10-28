use std::sync::Arc;

use crate::state::AppState;
use utoipa_axum::{router::OpenApiRouter, routes};

// API 路由器
pub fn api_router(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(crate::handlers::rooms::create))
        .routes(routes!(crate::handlers::rooms::find))
        .routes(routes!(crate::handlers::rooms::delete))
        .routes(routes!(crate::handlers::rooms::update_permissions))
        .routes(routes!(crate::handlers::rooms::update_room_settings))
        .routes(routes!(crate::handlers::rooms::issue_token))
        .routes(routes!(crate::handlers::rooms::list_tokens))
        .routes(routes!(crate::handlers::rooms::validate_token))
        .routes(routes!(crate::handlers::rooms::revoke_token))
        .routes(routes!(crate::handlers::content::list_contents))
        .routes(routes!(crate::handlers::content::prepare_upload))
        .routes(routes!(crate::handlers::content::upload_contents))
        .routes(routes!(crate::handlers::content::delete_contents))
        .routes(routes!(crate::handlers::content::download_content))
        .routes(routes!(crate::handlers::content::update_content))
        .routes(routes!(
            crate::handlers::chunked_upload::prepare_chunked_upload
        ))
        .routes(routes!(crate::handlers::chunked_upload::upload_chunk))
        .routes(routes!(crate::handlers::chunked_upload::get_upload_status))
        .routes(routes!(
            crate::handlers::chunked_upload::complete_file_merge
        ))
        .with_state(app_state)
}
