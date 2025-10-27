pub mod auth;
pub mod room;
pub mod status;

use utoipa_scalar::{Scalar, Servable};

pub const API_PREFIX: &str = "/api/v1";

pub fn scalar(api: utoipa::openapi::OpenApi) -> (Scalar<utoipa::openapi::OpenApi>, String) {
    let path = format!("{}/scalar", API_PREFIX);
    (Scalar::with_url(path.clone(), api), path)
}
