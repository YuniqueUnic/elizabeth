use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::state::AppState;

pub fn api_router(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(crate::handlers::admin::list_full_unbounded_rooms))
        .routes(routes!(crate::handlers::admin::run_room_gc))
        .routes(routes!(crate::handlers::admin::admin_stats))
        .routes(routes!(crate::handlers::admin::admin_list_rooms))
        .routes(routes!(crate::handlers::admin::admin_room_detail))
        .routes(routes!(crate::handlers::admin::admin_update_room))
        .routes(routes!(crate::handlers::admin::admin_delete_room))
        .routes(routes!(crate::handlers::admin::admin_mint_identity_code))
        .routes(routes!(crate::handlers::admin::admin_storage))
        .routes(routes!(crate::handlers::admin::admin_config))
        .routes(routes!(crate::handlers::admin::admin_update_runtime_config))
        .routes(routes!(crate::handlers::admin_auth::admin_login))
        .routes(routes!(crate::handlers::admin_auth::admin_me))
        .routes(routes!(crate::handlers::admin_auth::admin_change_password))
        .routes(routes!(crate::handlers::admin_auth::admin_list_api_keys))
        .routes(routes!(crate::handlers::admin_auth::admin_create_api_key))
        .routes(routes!(crate::handlers::admin_auth::admin_revoke_api_key))
        .with_state(app_state)
}
