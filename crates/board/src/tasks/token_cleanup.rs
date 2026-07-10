use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::scheduler::{ScheduledTask, ScheduledTaskId, TaskRunReport};
use crate::services::RefreshTokenService;

pub struct TokenCleanupTask {
    service: Arc<RefreshTokenService>,
}

impl TokenCleanupTask {
    pub fn new(service: Arc<RefreshTokenService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl ScheduledTask for TokenCleanupTask {
    fn id(&self) -> ScheduledTaskId {
        ScheduledTaskId::AuthTokenCleanup
    }

    async fn run(&self) -> Result<TaskRunReport> {
        let changed = self.service.cleanup_expired().await?;
        Ok(TaskRunReport {
            examined: changed,
            changed,
        })
    }
}
