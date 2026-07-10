use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use crate::dto::config::{PublicConfigResponse, PublicRoomConfig, PublicRoomExpiryConfig};
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/config",
    responses(
        (status = 200, description = "Public deployment configuration", body = PublicConfigResponse)
    ),
    tag = "config"
)]
pub async fn get_public_config(
    State(app_state): State<Arc<AppState>>,
) -> Json<PublicConfigResponse> {
    let expiry = app_state.room_expiry_policy();
    Json(PublicConfigResponse {
        room: PublicRoomConfig {
            expiry: PublicRoomExpiryConfig {
                allowed_ages_seconds: expiry.allowed_ages_seconds().to_vec(),
                default_age_seconds: expiry.default_age_seconds(),
            },
        },
    })
}
