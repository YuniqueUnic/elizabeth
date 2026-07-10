use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use async_trait::async_trait;
use axum::Router;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};

use crate::scheduler::{ScheduledTask, ScheduledTaskId, TaskRunReport};

pub use configrs::RateLimitConfig;

pub(crate) struct RateLimitSetup<S> {
    pub router: Router<S>,
    pub cleanup_task: Option<RateLimitCleanupRegistration>,
}

pub(crate) struct RateLimitCleanupRegistration {
    pub interval: Duration,
    pub task: Arc<dyn ScheduledTask>,
}

struct RateLimitCleanupTask {
    cleanup: Arc<dyn Fn() -> TaskRunReport + Send + Sync>,
}

#[async_trait]
impl ScheduledTask for RateLimitCleanupTask {
    fn id(&self) -> ScheduledTaskId {
        ScheduledTaskId::RateLimitCleanup
    }

    async fn run(&self) -> Result<TaskRunReport> {
        Ok((self.cleanup)())
    }
}

/// Apply rate limiting and return its supervised housekeeping task separately.
pub(crate) fn apply_rate_limit_layer<S>(
    config: &RateLimitConfig,
    router: Router<S>,
) -> Result<RateLimitSetup<S>>
where
    S: Clone + Send + Sync + 'static,
{
    if !config.enabled {
        logrs::info!("Rate limiting middleware disabled");
        return Ok(RateLimitSetup {
            router,
            cleanup_task: None,
        });
    }

    validate_config(config)?;
    logrs::info!(
        "Applying rate limiting middleware: {} req/sec, burst: {}, cleanup: {}s",
        config.per_second,
        config.burst_size,
        config.cleanup_interval_seconds
    );

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(config.per_second)
            .burst_size(config.burst_size as u32)
            .use_headers()
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .ok_or_else(|| anyhow::anyhow!("Failed to create rate limiter configuration"))?,
    );

    let governor_limiter = governor_conf.limiter().clone();
    let cleanup = Arc::new(move || {
        let before = governor_limiter.len() as u64;
        governor_limiter.retain_recent();
        let after = governor_limiter.len() as u64;
        if before > 0 {
            logrs::debug!(
                "Rate limiting storage cleanup: before={}, after={}",
                before,
                after
            );
        }
        TaskRunReport {
            examined: before,
            changed: before.saturating_sub(after),
        }
    });

    logrs::info!(
        "Rate limiting enabled: {} req/sec, burst: {}",
        config.per_second,
        config.burst_size
    );

    Ok(RateLimitSetup {
        router: router.layer(GovernorLayer::new(governor_conf)),
        cleanup_task: Some(RateLimitCleanupRegistration {
            interval: Duration::from_secs(config.cleanup_interval_seconds),
            task: Arc::new(RateLimitCleanupTask { cleanup }),
        }),
    })
}

fn validate_config(config: &RateLimitConfig) -> Result<()> {
    if config.per_second == 0 {
        bail!("Rate limiting per_second must be greater than zero");
    }
    if config.burst_size == 0 {
        bail!("Rate limiting burst_size must be greater than zero");
    }
    if config.burst_size > u64::from(u32::MAX) {
        bail!("Rate limiting burst_size exceeds the supported u32 range");
    }
    if config.cleanup_interval_seconds == 0 {
        bail!("Rate limiting cleanup interval must be greater than zero");
    }
    Ok(())
}
