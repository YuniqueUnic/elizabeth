use axum_responses::http::HttpResponse;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::route::API_PREFIX;

#[utoipa::path(
    get,
    path = format!("{}/health", API_PREFIX),
    responses(
        (status = 200, description = "Service is running")
    )
)]
#[axum_macros::debug_handler]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok()
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct Status {
    uptime: String,
    memory: String,
    cpu: String,
}

#[utoipa::path(
    get,
    path = format!("{}/status", API_PREFIX),
    responses(
        (status = 200, description = "The status of service", body = ())
    )
)]
#[axum_macros::debug_handler]
pub async fn status() -> HttpResponse {
    let status = Status {
        uptime: Utc::now().to_rfc3339(),
        ..Default::default()
    };
    HttpResponse::Ok().data(status)
}
