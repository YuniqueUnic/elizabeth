use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::errors::AppError;

/// Failed attempt record for a specific client & content
#[derive(Debug, Clone)]
struct AttemptRecord {
    failed_count: u32,
    last_failed_at: Instant,
    locked_until: Option<Instant>,
}

/// Anti-brute-force limiter for access code redemption
#[derive(Debug, Default)]
pub struct AccessCodeLimiter {
    records: RwLock<HashMap<(i64, String), AttemptRecord>>,
}

impl AccessCodeLimiter {
    /// Window duration for resetting failure count (if no failures occur)
    const ATTEMPT_WINDOW: Duration = Duration::from_secs(60);
    /// Maximum failures allowed before first lockout
    const MAX_ALLOWED_FAILURES: u32 = 5;

    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
        }
    }

    /// Check if client is currently rate limited / locked out from trying codes for this content
    pub fn check_rate_limit(&self, content_id: i64, client_id: &str) -> Result<(), AppError> {
        let key = (content_id, client_id.to_string());
        let now = Instant::now();

        let Ok(records) = self.records.read() else {
            return Ok(());
        };

        if let Some(locked_until) = records
            .get(&key)
            .and_then(|r| r.locked_until)
            .filter(|&locked_until| now < locked_until)
        {
            let remaining_secs = (locked_until - now).as_secs().max(1);
            return Err(AppError::too_many_requests(format!(
                "Too many failed attempts. Please wait {} seconds before trying again.",
                remaining_secs
            )));
        }

        Ok(())
    }

    /// Record a failed attempt. Returns current total failed count and remaining lock seconds if locked.
    pub fn record_failure(&self, content_id: i64, client_id: &str) -> (u32, Option<u64>) {
        let key = (content_id, client_id.to_string());
        let now = Instant::now();

        let Ok(mut records) = self.records.write() else {
            return (1, None);
        };

        // Periodic cleanup if map grows too large
        if records.len() > 5000 {
            records.retain(|_, v| {
                if let Some(locked) = v.locked_until {
                    locked > now
                } else {
                    now.duration_since(v.last_failed_at) < Duration::from_secs(3600)
                }
            });
        }

        let record = records.entry(key).or_insert_with(|| AttemptRecord {
            failed_count: 0,
            last_failed_at: now,
            locked_until: None,
        });

        // If last failure was long ago outside the attempt window and not locked, reset count
        if record.locked_until.is_none_or(|l| now >= l)
            && now.duration_since(record.last_failed_at) > Self::ATTEMPT_WINDOW
        {
            record.failed_count = 0;
        }

        record.failed_count += 1;
        record.last_failed_at = now;

        // Compute lockout if threshold exceeded
        if record.failed_count >= Self::MAX_ALLOWED_FAILURES {
            let lock_duration = if record.failed_count >= 15 {
                Duration::from_secs(1800) // 30 minutes
            } else if record.failed_count >= 10 {
                Duration::from_secs(300) // 5 minutes
            } else {
                Duration::from_secs(60) // 1 minute
            };

            record.locked_until = Some(now + lock_duration);
            return (record.failed_count, Some(lock_duration.as_secs()));
        }

        (record.failed_count, None)
    }

    /// Record a successful redemption and clear any failed history for this client
    pub fn record_success(&self, content_id: i64, client_id: &str) {
        let key = (content_id, client_id.to_string());
        let Ok(mut records) = self.records.write() else {
            return;
        };
        records.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_allows_up_to_max_failures_then_locks() {
        let limiter = AccessCodeLimiter::new();
        let content_id = 100;
        let client_id = "test_client_1";

        // Initial check should pass
        assert!(limiter.check_rate_limit(content_id, client_id).is_ok());

        // First 4 failures should not lock
        for i in 1..=4 {
            let (count, locked) = limiter.record_failure(content_id, client_id);
            assert_eq!(count, i);
            assert_eq!(locked, None);
            assert!(limiter.check_rate_limit(content_id, client_id).is_ok());
        }

        // 5th failure triggers 60s lockout
        let (count, locked) = limiter.record_failure(content_id, client_id);
        assert_eq!(count, 5);
        assert_eq!(locked, Some(60));

        // Now check should fail with TooManyRequests
        let check = limiter.check_rate_limit(content_id, client_id);
        assert!(check.is_err());
        let err = check.unwrap_err();
        assert_eq!(err.status_code(), axum::http::StatusCode::TOO_MANY_REQUESTS);

        // Success clears the record
        limiter.record_success(content_id, client_id);
        assert!(limiter.check_rate_limit(content_id, client_id).is_ok());
    }

    #[test]
    fn test_different_contents_and_clients_are_isolated() {
        let limiter = AccessCodeLimiter::new();

        // Lock client A on content 1
        for _ in 0..5 {
            limiter.record_failure(1, "client_a");
        }
        assert!(limiter.check_rate_limit(1, "client_a").is_err());

        // Client A on content 2 should still be allowed
        assert!(limiter.check_rate_limit(2, "client_a").is_ok());

        // Client B on content 1 should still be allowed
        assert!(limiter.check_rate_limit(1, "client_b").is_ok());
    }
}
