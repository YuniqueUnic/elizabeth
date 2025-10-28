use axum::{Router, response::IntoResponse};
use logrs::error;
use std::sync::Arc;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};

// Re-export RateLimitConfig from configrs
pub use configrs::RateLimitConfig;

/// Apply rate limiting middleware to the router
pub fn apply_rate_limit_layer<S>(config: &RateLimitConfig, router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !config.enabled {
        logrs::info!("Rate limiting middleware disabled");
        return router;
    }

    logrs::info!(
        "Applying rate limiting middleware: {} req/sec, burst: {}, cleanup: {}s",
        config.per_second,
        config.burst_size,
        config.cleanup_interval_seconds
    );

    // Validate configuration
    if config.per_second == 0 {
        logrs::warn!("Rate limiting per_second cannot be 0, using 1");
        return router;
    }
    if config.burst_size == 0 {
        logrs::warn!("Rate limiting burst_size cannot be 0, using 1");
        return router;
    }

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(config.per_second)
            .burst_size(config.burst_size as u32)
            .use_headers()
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("Failed to create rate limiter configuration"),
    );

    // Start background cleanup task
    let governor_limiter = governor_conf.limiter().clone();
    let cleanup_interval = config.cleanup_interval_seconds;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(cleanup_interval));
        loop {
            interval.tick().await;
            let size = governor_limiter.len();
            if size > 0 {
                logrs::debug!("Rate limiting storage size: {}", size);
            }
            governor_limiter.retain_recent();
        }
    });

    logrs::info!(
        "Rate limiting enabled: {} req/sec, burst: {}",
        config.per_second,
        config.burst_size
    );

    router.layer(GovernorLayer::new(governor_conf))
}
