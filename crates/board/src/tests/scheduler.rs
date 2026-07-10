use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use axum::Router;
use tokio::sync::Notify;

use crate::middleware::rate_limit::apply_rate_limit_layer;
use crate::scheduler::{
    ScheduledTask, ScheduledTaskId, TaskRegistration, TaskRunReport, TaskScheduler,
};

struct RecordingTask {
    id: ScheduledTaskId,
    runs: Arc<AtomicU64>,
    notify: Arc<Notify>,
    fail: bool,
}

#[async_trait]
impl ScheduledTask for RecordingTask {
    fn id(&self) -> ScheduledTaskId {
        self.id
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
                id: ScheduledTaskId::RoomLifecycle,
                runs: successful_runs.clone(),
                notify: successful_notify.clone(),
                fail: false,
            }),
        },
        TaskRegistration {
            interval: Duration::from_secs(3600),
            timeout: Duration::from_secs(1),
            task: Arc::new(RecordingTask {
                id: ScheduledTaskId::AuthTokenCleanup,
                runs: failing_runs.clone(),
                notify: failing_notify.clone(),
                fail: true,
            }),
        },
    ])
    .expect("valid scheduler registrations")
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

struct BlockingTask {
    started: Arc<Notify>,
}

#[async_trait]
impl ScheduledTask for BlockingTask {
    fn id(&self) -> ScheduledTaskId {
        ScheduledTaskId::RoomLifecycle
    }

    async fn run(&self) -> Result<TaskRunReport> {
        self.started.notify_one();
        std::future::pending().await
    }
}

#[tokio::test]
async fn scheduler_shutdown_cancels_an_in_flight_task() {
    let started = Arc::new(Notify::new());
    let scheduler = TaskScheduler::new(vec![TaskRegistration {
        interval: Duration::from_secs(3600),
        timeout: Duration::from_secs(300),
        task: Arc::new(BlockingTask {
            started: started.clone(),
        }),
    }])
    .expect("valid scheduler registration")
    .start();

    tokio::time::timeout(Duration::from_secs(1), started.notified())
        .await
        .expect("blocking task should start immediately");
    tokio::time::timeout(Duration::from_millis(200), scheduler.shutdown())
        .await
        .expect("shutdown should cancel the in-flight task");
}

#[test]
fn scheduler_rejects_zero_intervals_and_duplicate_task_ids() {
    let task = || {
        Arc::new(RecordingTask {
            id: ScheduledTaskId::RoomLifecycle,
            runs: Arc::new(AtomicU64::new(0)),
            notify: Arc::new(Notify::new()),
            fail: false,
        }) as Arc<dyn ScheduledTask>
    };

    let zero_interval = TaskScheduler::new(vec![TaskRegistration {
        interval: Duration::ZERO,
        timeout: Duration::from_secs(1),
        task: task(),
    }]);
    assert!(zero_interval.is_err());

    let duplicate_ids = TaskScheduler::new(vec![
        TaskRegistration {
            interval: Duration::from_secs(1),
            timeout: Duration::from_secs(1),
            task: task(),
        },
        TaskRegistration {
            interval: Duration::from_secs(2),
            timeout: Duration::from_secs(1),
            task: task(),
        },
    ]);
    assert!(duplicate_ids.is_err());
}

#[tokio::test]
async fn rate_limit_cleanup_is_registered_only_when_enabled() {
    let disabled = configrs::RateLimitConfig::default();
    let disabled_setup = apply_rate_limit_layer(&disabled, Router::<()>::new())
        .expect("disabled rate limit should be valid");
    assert!(disabled_setup.cleanup_task.is_none());

    let enabled = configrs::RateLimitConfig {
        enabled: true,
        per_second: 10,
        burst_size: 20,
        cleanup_interval_seconds: 60,
    };
    let enabled_setup = apply_rate_limit_layer(&enabled, Router::<()>::new())
        .expect("enabled rate limit should be valid");
    let cleanup = enabled_setup
        .cleanup_task
        .expect("enabled rate limit should register cleanup");
    assert_eq!(cleanup.interval, Duration::from_secs(60));
    assert_eq!(cleanup.task.id(), ScheduledTaskId::RateLimitCleanup);
    assert_eq!(cleanup.task.run().await.expect("cleanup run").changed, 0);

    let invalid = configrs::RateLimitConfig {
        cleanup_interval_seconds: 0,
        ..enabled
    };
    assert!(apply_rate_limit_layer(&invalid, Router::<()>::new()).is_err());
}
