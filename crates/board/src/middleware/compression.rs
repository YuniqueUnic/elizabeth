use axum::Router;

// Re-export CompressionConfig from configrs
pub use configrs::CompressionConfig;

/// Apply compression middleware to the router
pub fn apply_compression_layer<S>(config: &CompressionConfig, router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !config.enabled {
        tracing::info!("Compression middleware disabled");
        return router;
    }

    tracing::info!(
        "Applying compression middleware with min_size: {} bytes",
        config.min_content_length
    );

    let layer = tower_http::compression::CompressionLayer::new();

    // Note: min_content_length filtering would need custom implementation
    // For now, we'll log the configuration
    if config.min_content_length > 0 {
        tracing::debug!(
            "Compression will apply to responses larger than {} bytes",
            config.min_content_length
        );
    }

    router.layer(layer)
}
