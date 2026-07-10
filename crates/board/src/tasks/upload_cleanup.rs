use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::repository::{IRoomUploadReservationRepository, RoomUploadReservationRepository};
use crate::scheduler::{ScheduledTask, ScheduledTaskId, TaskRunReport};

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
    fn id(&self) -> ScheduledTaskId {
        ScheduledTaskId::UploadReservationCleanup
    }

    async fn run(&self) -> Result<TaskRunReport> {
        let chunked_ids = self.repository.list_expired_chunked_ids().await?;
        for reservation_id in chunked_ids {
            crate::chunk_temp_storage::remove_reservation_dir(reservation_id).await?;
        }
        let changed = self.repository.purge_expired().await?;
        Ok(TaskRunReport {
            examined: changed,
            changed,
        })
    }
}
