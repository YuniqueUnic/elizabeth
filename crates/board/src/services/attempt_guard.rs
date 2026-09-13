use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::errors::AppError;

/// 防爆破作用域：同一 (作用域，客户端键) 共享失败计数与锁定状态。
/// 每个信任边界（身份码兑换、房间密码、文件兑换码、管理登录）各占一个作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuardScope {
    /// 房间身份码兑换（按房间隔离）
    IdentityCode(i64),
    /// 房间密码校验（按房间隔离）
    RoomPassword(i64),
    /// 文件下载兑换码（按内容隔离）
    AccessCode(i64),
    /// 平台管理 API 凭证校验
    AdminLogin,
}

/// 单个 (作用域，客户端) 的失败记录
#[derive(Debug, Clone)]
struct AttemptRecord {
    failed_count: u32,
    last_failed_at: Instant,
    locked_until: Option<Instant>,
}

/// 统一防爆破守卫：滑动窗口内累计失败次数，超过阈值按次数升级锁定
/// （5 次 → 1 分钟，10 次 → 5 分钟，15 次 → 30 分钟）；成功后清除记录。
#[derive(Debug, Default)]
pub struct AttemptGuard {
    records: RwLock<HashMap<(GuardScope, String), AttemptRecord>>,
}

impl AttemptGuard {
    /// 无失败的滑动窗口：窗口外未锁定的失败记录重置计数
    const ATTEMPT_WINDOW: Duration = Duration::from_secs(60);
    /// 触发首次锁定的失败次数阈值
    const MAX_ALLOWED_FAILURES: u32 = 5;
    /// 记录数上限，超过时清理过期记录，防止无界增长
    const MAX_RECORDS: usize = 5000;
    /// 未锁定记录的保留时长
    const RECORD_TTL: Duration = Duration::from_secs(3600);

    pub fn new() -> Self {
        Self::default()
    }

    /// 若客户端处于锁定期，返回 TooManyRequests；否则放行。
    pub fn check(&self, scope: GuardScope, client_key: &str) -> Result<(), AppError> {
        let now = Instant::now();
        let Ok(records) = self.records.read() else {
            return Ok(());
        };

        if let Some(locked_until) = records
            .get(&(scope, client_key.to_string()))
            .and_then(|r| r.locked_until)
            .filter(|&locked_until| now < locked_until)
        {
            let remaining_secs = (locked_until - now).as_secs().max(1);
            return Err(AppError::too_many_requests(format!(
                "Too many failed attempts. Please wait {remaining_secs} seconds before trying again."
            )));
        }

        Ok(())
    }

    /// 记录一次失败；达到阈值时计算锁定时长，返回 (累计失败次数，锁定秒数)。
    pub fn record_failure(&self, scope: GuardScope, client_key: &str) -> (u32, Option<u64>) {
        let now = Instant::now();
        let Ok(mut records) = self.records.write() else {
            return (1, None);
        };

        if records.len() > Self::MAX_RECORDS {
            records.retain(|_, v| match v.locked_until {
                Some(locked) => locked > now,
                None => now.duration_since(v.last_failed_at) < Self::RECORD_TTL,
            });
        }

        let record = records
            .entry((scope, client_key.to_string()))
            .or_insert_with(|| AttemptRecord {
                failed_count: 0,
                last_failed_at: now,
                locked_until: None,
            });

        if record.locked_until.is_none_or(|l| now >= l)
            && now.duration_since(record.last_failed_at) > Self::ATTEMPT_WINDOW
        {
            record.failed_count = 0;
        }

        record.failed_count += 1;
        record.last_failed_at = now;

        if record.failed_count >= Self::MAX_ALLOWED_FAILURES {
            let lock_duration = match record.failed_count {
                15.. => Duration::from_secs(1800),
                10.. => Duration::from_secs(300),
                _ => Duration::from_secs(60),
            };
            record.locked_until = Some(now + lock_duration);
            return (record.failed_count, Some(lock_duration.as_secs()));
        }

        (record.failed_count, None)
    }

    /// 校验成功：清除该客户端在此作用域的失败记录。
    pub fn record_success(&self, scope: GuardScope, client_key: &str) {
        let Ok(mut records) = self.records.write() else {
            return;
        };
        records.remove(&(scope, client_key.to_string()));
    }
}
