pub mod compression;
pub mod cors;
pub mod rate_limit;
pub mod request_id;
pub mod security;
pub mod tracing;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum::{Router, extract::DefaultBodyLimit};

use crate::scheduler::ScheduledTask;

/// Re-export middleware configuration types from configrs
pub use configrs::{
    CompressionConfig, CorsConfig, MiddlewareConfig, RateLimitConfig, RequestIdConfig,
    SecurityConfig, TracingConfig,
};

pub(crate) struct MiddlewareScheduledTask {
    pub interval: Duration,
    pub task: Arc<dyn ScheduledTask>,
}

pub(crate) struct MiddlewareSetup {
    pub router: Router,
    pub scheduled_tasks: Vec<MiddlewareScheduledTask>,
}

/// Apply all configured middleware and return supervised housekeeping tasks.
pub(crate) fn apply(
    middleware_config: &MiddlewareConfig,
    router: Router,
) -> Result<MiddlewareSetup> {
    let router = tracing::apply_tracing_layer(&middleware_config.tracing, router);
    let router = request_id::apply_request_id_layer(&middleware_config.request_id, router);
    let router = compression::apply_compression_layer(&middleware_config.compression, router);
    let router = cors::apply_cors_layer(&middleware_config.cors, router);
    let router = security::apply_security_layer(&middleware_config.security, router);
    let router = router.layer(DefaultBodyLimit::max(
        crate::constants::upload::MAX_MULTIPART_BODY_SIZE,
    ));

    let rate_limit = rate_limit::apply_rate_limit_layer(&middleware_config.rate_limit, router)?;
    let scheduled_tasks = rate_limit
        .cleanup_task
        .into_iter()
        .map(|cleanup| MiddlewareScheduledTask {
            interval: cleanup.interval,
            task: cleanup.task,
        })
        .collect();
    Ok(MiddlewareSetup {
        router: rate_limit.router,
        scheduled_tasks,
    })
}

/// Create middleware configuration from application config
pub fn from_app_config(app_config: &configrs::Config) -> MiddlewareConfig {
    // Extract middleware configuration from the main app config
    app_config.app.middleware.clone()
}
