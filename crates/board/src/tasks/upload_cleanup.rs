use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::repository::{IRoomUploadReservationRepository, RoomUploadReservationRepository};
use crate::scheduler::{ScheduledTask, TaskRunReport};

pub struct UploadCleanupTask {
    repository: Arc<RoomUploadReservationRepository>,
}

impl UploadCleanupTask {
    pub fn new(repository: Arc<RoomUploadReservationRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ScheduledTask for UploadCleanupTask {
    fn name(&self) -> &'static str {
        "upload_reservation_cleanup"
    }

    async fn run(&self) -> Result<TaskRunReport> {
        let changed = self.repository.purge_expired().await?;
        Ok(TaskRunReport {
            examined: changed,
            changed,
        })
    }
}
