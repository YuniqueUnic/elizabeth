use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::scheduler::{ScheduledTask, ScheduledTaskId, TaskRunReport};
use crate::services::RoomLifecycleService;
use crate::websocket::connection::ConnectionManager;

pub struct RoomLifecycleTask {
    service: Arc<RoomLifecycleService>,
    connections: Arc<ConnectionManager>,
    batch_limit: u32,
    private_name_lock_seconds: i64,
}

impl RoomLifecycleTask {
    pub fn new(
        service: Arc<RoomLifecycleService>,
        connections: Arc<ConnectionManager>,
        batch_limit: u32,
        private_name_lock_seconds: i64,
    ) -> Self {
        Self {
            service,
            connections,
            batch_limit,
            private_name_lock_seconds,
        }
    }
}

#[async_trait]
impl ScheduledTask for RoomLifecycleTask {
    fn id(&self) -> ScheduledTaskId {
        ScheduledTaskId::RoomLifecycle
    }

    async fn run(&self) -> Result<TaskRunReport> {
        let report = self
            .service
            .run(
                &self.connections,
                self.batch_limit,
                self.private_name_lock_seconds,
            )
            .await?;
        Ok(TaskRunReport {
            examined: u64::from(self.batch_limit) * 2,
            changed: report.changed(),
        })
    }
}
