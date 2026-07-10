use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskRunReport {
    pub examined: u64,
    pub changed: u64,
}

#[async_trait]
pub trait ScheduledTask: Send + Sync {
    fn name(&self) -> &'static str;
    async fn run(&self) -> Result<TaskRunReport>;
}

pub struct TaskRegistration {
    pub interval: Duration,
    pub timeout: Duration,
    pub task: Arc<dyn ScheduledTask>,
}

pub struct TaskScheduler {
    registrations: Vec<TaskRegistration>,
}

impl TaskScheduler {
    pub fn new(registrations: Vec<TaskRegistration>) -> Self {
        Self { registrations }
    }

    pub fn start(self) -> SchedulerHandle {
        let cancellation = CancellationToken::new();
        let tracker = TaskTracker::new();
        for registration in self.registrations {
            let cancellation = cancellation.child_token();
            tracker.spawn(run_registration(registration, cancellation));
        }
        tracker.close();
        SchedulerHandle {
            cancellation,
            tracker,
        }
    }
}

pub struct SchedulerHandle {
    cancellation: CancellationToken,
    tracker: TaskTracker,
}

impl SchedulerHandle {
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub async fn shutdown(self) {
        self.cancellation.cancel();
        self.tracker.wait().await;
    }
}

async fn run_registration(registration: TaskRegistration, cancellation: CancellationToken) {
    let mut interval = tokio::time::interval(registration.interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            () = cancellation.cancelled() => break,
            _ = interval.tick() => {
                let started = Instant::now();
                match tokio::time::timeout(registration.timeout, registration.task.run()).await {
                    Ok(Ok(report)) => log::info!(
                        "Scheduled task '{}' completed in {:?}: examined={}, changed={}",
                        registration.task.name(),
                        started.elapsed(),
                        report.examined,
                        report.changed,
                    ),
                    Ok(Err(error)) => log::warn!(
                        "Scheduled task '{}' failed after {:?}: {error}",
                        registration.task.name(),
                        started.elapsed(),
                    ),
                    Err(_) => log::warn!(
                        "Scheduled task '{}' timed out after {:?}",
                        registration.task.name(),
                        registration.timeout,
                    ),
                }
            }
        }
    }
}
