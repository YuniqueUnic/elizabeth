pub mod room;
pub mod status;

use axum::Json;

use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::build;

pub const API_PREFIX: &str = "/api/v1";

#[derive(OpenApi)]
#[openapi(
    info(
        title = "API Docs",
        description = "API documentation for the server",
        version = build::PKG_VERSION
    ),
    tags((name=API_PREFIX, description="API v1")),
    paths(openapi)
)]
pub struct ApiDoc;

/// Return JSON version of an OpenAPI schema
#[utoipa::path(
    get,
    path = format!("{}/openapi.json", API_PREFIX),
    responses(
        (status = 200, description = "JSON file", body = ())
    )
)]
pub async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

pub fn scalar(api: utoipa::openapi::OpenApi) -> (Scalar<utoipa::openapi::OpenApi>, String) {
    let path = format!("{}/scalar", API_PREFIX);
    (Scalar::with_url(path.clone(), api), path)
}
