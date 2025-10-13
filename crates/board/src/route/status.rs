use axum::response::IntoResponse;
use axum_responses::http::HttpResponse;
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
    )
)]
pub async fn health() -> impl IntoResponse {
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
pub async fn status() -> impl IntoResponse {
    let status = Status {
        uptime: Utc::now().to_rfc3339(),
        ..Default::default()
    };
    HttpResponse::Ok().data(status)
}
