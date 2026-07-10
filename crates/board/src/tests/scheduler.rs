use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::Notify;

use crate::scheduler::{ScheduledTask, TaskRegistration, TaskRunReport, TaskScheduler};

struct RecordingTask {
    name: &'static str,
    runs: Arc<AtomicU64>,
    notify: Arc<Notify>,
    fail: bool,
}

#[async_trait]
impl ScheduledTask for RecordingTask {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn run(&self) -> Result<TaskRunReport> {
        let runs = self.runs.fetch_add(1, Ordering::SeqCst) + 1;
        self.notify.notify_one();
        if self.fail {
            return Err(anyhow!("expected task failure"));
        }
        Ok(TaskRunReport {
            examined: runs,
            changed: runs,
        })
    }
}

#[tokio::test]
async fn scheduler_runs_tasks_immediately_and_isolates_failures() {
    let successful_runs = Arc::new(AtomicU64::new(0));
    let successful_notify = Arc::new(Notify::new());
    let failing_runs = Arc::new(AtomicU64::new(0));
    let failing_notify = Arc::new(Notify::new());

    let scheduler = TaskScheduler::new(vec![
        TaskRegistration {
            interval: Duration::from_secs(3600),
            timeout: Duration::from_secs(1),
            task: Arc::new(RecordingTask {
                name: "successful",
                runs: successful_runs.clone(),
                notify: successful_notify.clone(),
                fail: false,
            }),
        },
        TaskRegistration {
            interval: Duration::from_secs(3600),
            timeout: Duration::from_secs(1),
            task: Arc::new(RecordingTask {
                name: "failing",
                runs: failing_runs.clone(),
                notify: failing_notify.clone(),
                fail: true,
            }),
        },
    ])
    .start();

    tokio::time::timeout(Duration::from_secs(1), successful_notify.notified())
        .await
        .expect("successful task should run on startup");
    tokio::time::timeout(Duration::from_secs(1), failing_notify.notified())
        .await
        .expect("failing task should run on startup");
    assert_eq!(successful_runs.load(Ordering::SeqCst), 1);
    assert_eq!(failing_runs.load(Ordering::SeqCst), 1);

    scheduler.shutdown().await;
}
