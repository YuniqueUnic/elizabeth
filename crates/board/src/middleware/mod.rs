pub mod compression;
pub mod cors;
pub mod rate_limit;
pub mod request_id;
pub mod security;
pub mod tracing;

use axum::{Router, extract::DefaultBodyLimit};

/// Re-export middleware configuration types from configrs
pub use configrs::{
    CompressionConfig, CorsConfig, MiddlewareConfig, RateLimitConfig, RequestIdConfig,
    SecurityConfig, TracingConfig,
};

/// Apply all configured middleware to the router
pub fn apply(middleware_config: &MiddlewareConfig, router: axum::Router) -> axum::Router {
    let router = tracing::apply_tracing_layer(&middleware_config.tracing, router);
    let router = request_id::apply_request_id_layer(&middleware_config.request_id, router);
    let router = compression::apply_compression_layer(&middleware_config.compression, router);
    let router = cors::apply_cors_layer(&middleware_config.cors, router);
    let router = security::apply_security_layer(&middleware_config.security, router);
    let router = router.layer(DefaultBodyLimit::max(
        crate::constants::upload::MAX_MULTIPART_BODY_SIZE,
    ));

    rate_limit::apply_rate_limit_layer(&middleware_config.rate_limit, router)
}

/// Create middleware configuration from application config
pub fn from_app_config(app_config: &configrs::Config) -> MiddlewareConfig {
    // Extract middleware configuration from the main app config
    app_config.app.middleware.clone()
}
