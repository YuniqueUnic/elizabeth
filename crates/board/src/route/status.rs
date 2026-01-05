use axum::{Json, response::IntoResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::route::API_PREFIX;

pub fn api_router() -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(status))
}

#[utoipa::path(
    get,
    path = format!("{}/health", API_PREFIX),
    responses(
        (status = 200, description = "Service is running")
    ),
    tag = "status"
)]
pub async fn health() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "OK")
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
        (status = 200, description = "The status of service", body = Status)
    ),
    tag = "status"
)]
pub async fn status() -> impl IntoResponse {
    let status = Status {
        uptime: Utc::now().to_rfc3339(),
        ..Default::default()
    };
    Json(status)
}
