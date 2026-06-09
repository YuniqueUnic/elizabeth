use std::sync::Arc;

use crate::state::AppState;
use utoipa_axum::{router::OpenApiRouter, routes};

// API 路由器
pub fn api_router(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(crate::handlers::rooms::lifecycle::create))
        .routes(routes!(crate::handlers::rooms::lifecycle::find))
        .routes(routes!(crate::handlers::rooms::lifecycle::delete))
        .routes(routes!(
            crate::handlers::rooms::permissions::update_permissions
        ))
        .routes(routes!(
            crate::handlers::rooms::settings::update_room_settings
        ))
        .routes(routes!(crate::handlers::rooms::tokens::issue_token))
        .routes(routes!(crate::handlers::rooms::tokens::list_tokens))
        .routes(routes!(crate::handlers::rooms::tokens::validate_token))
        .routes(routes!(crate::handlers::rooms::tokens::revoke_token))
        .routes(routes!(crate::handlers::content::upload::list_contents))
        .routes(routes!(crate::handlers::content::upload::prepare_upload))
        .routes(routes!(crate::handlers::content::upload::upload_contents))
        .routes(routes!(crate::handlers::content::delete::delete_contents))
        .routes(routes!(
            crate::handlers::content::download::download_content_global
        ))
        .routes(routes!(crate::handlers::content::update::update_content))
        .routes(routes!(crate::handlers::content::url::create_url_content))
        .routes(routes!(crate::handlers::content::message::create_message))
        .routes(routes!(
            crate::handlers::chunked_upload::prepare_chunked_upload
        ))
        .routes(routes!(
            crate::handlers::chunked_upload::upload::upload_chunk
        ))
        .routes(routes!(crate::handlers::chunked_upload::get_upload_status))
        .routes(routes!(
            crate::handlers::chunked_upload::complete::complete_file_merge
        ))
        .routes(routes!(
            crate::handlers::chunked_upload::cancel_chunked_upload
        ))
        .with_state(app_state)
}
