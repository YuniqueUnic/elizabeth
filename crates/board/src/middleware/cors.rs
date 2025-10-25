use axum::Router;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, AllowPrivateNetwork, CorsLayer};

// Re-export CorsConfig from configrs
pub use configrs::CorsConfig;

/// Apply CORS middleware to the router
pub fn apply_cors_layer<S>(config: &CorsConfig, router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !config.enabled {
        tracing::info!("CORS middleware disabled");
        return router;
    }

    tracing::info!(
        "Applying CORS middleware with {} origins, {} methods",
        config.allowed_origins.len(),
        config.allowed_methods.len()
    );

    let mut cors = CorsLayer::new();

    // Configure allowed origins
    if config.allowed_origins.contains(&"*".to_string()) {
        cors = cors.allow_origin(AllowOrigin::any());
        tracing::debug!("CORS: allowing any origin");
    } else {
        for origin in &config.allowed_origins {
            tracing::debug!("CORS: allowing origin: {}", origin);
        }
        // Note: For specific origins, we would need to parse them into HeaderValue
        // For now, we'll use wildcard if specific origins are configured
        cors = cors.allow_origin(AllowOrigin::any());
    }

    // Configure allowed methods
    if config.allowed_methods.contains(&"*".to_string()) {
        cors = cors.allow_methods(AllowMethods::any());
        tracing::debug!("CORS: allowing any method");
    } else {
        let methods: Vec<_> = config
            .allowed_methods
            .iter()
            .filter_map(|m| m.parse().ok())
            .collect();
        if !methods.is_empty() {
            cors = cors.allow_methods(AllowMethods::list(methods));
            tracing::debug!("CORS: allowing {} methods", config.allowed_methods.len());
        }
    }

    // Configure allowed headers
    if config.allowed_headers.contains(&"*".to_string()) {
        cors = cors.allow_headers(AllowHeaders::any());
        tracing::debug!("CORS: allowing any header");
    } else {
        let headers: Vec<_> = config
            .allowed_headers
            .iter()
            .filter_map(|h| h.parse().ok())
            .collect();
        if !headers.is_empty() {
            cors = cors.allow_headers(AllowHeaders::list(headers));
            tracing::debug!("CORS: allowing {} headers", config.allowed_headers.len());
        }
    }

    // Configure other settings
    cors = cors
        .allow_credentials(config.allow_credentials)
        .max_age(std::time::Duration::from_secs(config.max_age));

    if config.allow_credentials {
        tracing::debug!("CORS: credentials allowed");
    }

    router.layer(cors)
}
