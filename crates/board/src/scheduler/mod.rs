use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use async_trait::async_trait;
use std::collections::HashSet;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskRunReport {
    pub examined: u64,
    pub changed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScheduledTaskId {
    RoomLifecycle,
    AuthTokenCleanup,
    UploadReservationCleanup,
    RateLimitCleanup,
}

impl std::fmt::Display for ScheduledTaskId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::RoomLifecycle => "room_lifecycle",
            Self::AuthTokenCleanup => "auth_token_cleanup",
            Self::UploadReservationCleanup => "upload_reservation_cleanup",
            Self::RateLimitCleanup => "rate_limit_cleanup",
        };
        formatter.write_str(value)
    }
}

#[async_trait]
pub trait ScheduledTask: Send + Sync {
    fn id(&self) -> ScheduledTaskId;
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
    pub fn new(registrations: Vec<TaskRegistration>) -> Result<Self> {
        let mut ids = HashSet::new();
        for registration in &registrations {
            if registration.interval.is_zero() {
                bail!(
                    "Scheduled task '{}' interval must be greater than zero",
                    registration.task.id()
                );
            }
            if registration.timeout.is_zero() {
                bail!(
                    "Scheduled task '{}' timeout must be greater than zero",
                    registration.task.id()
                );
            }
            if !ids.insert(registration.task.id()) {
                bail!(
                    "Scheduled task '{}' is registered more than once",
                    registration.task.id()
                );
            }
        }
        Ok(Self { registrations })
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
                let execution = tokio::time::timeout(
                    registration.timeout,
                    registration.task.run(),
                );
                let outcome = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => break,
                    outcome = execution => outcome,
                };
                match outcome {
                    Ok(Ok(report)) => log::info!(
                        "Scheduled task '{}' completed in {:?}: examined={}, changed={}",
                        registration.task.id(),
                        started.elapsed(),
                        report.examined,
                        report.changed,
                    ),
                    Ok(Err(error)) => log::warn!(
                        "Scheduled task '{}' failed after {:?}: {error}",
                        registration.task.id(),
                        started.elapsed(),
                    ),
                    Err(_) => log::warn!(
                        "Scheduled task '{}' timed out after {:?}",
                        registration.task.id(),
                        registration.timeout,
                    ),
                }
            }
        }
    }
}
