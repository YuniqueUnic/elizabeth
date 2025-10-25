use crate::errors::{AppError, AppResult};

/// 事务配置
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试延迟（毫秒）
    pub retry_delay_ms: u64,
    /// 事务超时时间（秒）
    pub timeout_seconds: u64,
    /// 是否启用批量事务优化
    pub enable_batch_optimization: bool,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 100,
            timeout_seconds: 30,
            enable_batch_optimization: true,
        }
    }
}

/// 事务工具函数
pub struct TransactionUtils;

impl TransactionUtils {
    /// 检查是否存在死锁
    pub fn is_deadlock_error(error: &sqlx::Error) -> bool {
        match error {
            sqlx::Error::Database(db_err) => {
                let message = db_err.message();
                message.contains("deadlock") || message.contains("lock wait timeout")
            }
            _ => false,
        }
    }

    /// 检查是否存在连接超时
    pub fn is_connection_timeout_error(error: &sqlx::Error) -> bool {
        matches!(error, sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed)
    }

    /// 创建事务错误上下文
    pub fn create_transaction_context(operation: &str, step: &str) -> String {
        format!("Transaction failed during {} - {}", operation, step)
    }

    /// 检查错误是否可重试
    pub fn is_retryable_error(error: &AppError) -> bool {
        match error {
            AppError::Database(db_err) => {
                matches!(db_err, sqlx::Error::Database(_) | sqlx::Error::PoolTimedOut)
            }
            _ => false,
        }
    }
}

/// 连接池监控
pub struct ConnectionPoolMonitor;

impl ConnectionPoolMonitor {
    /// 监控连接池状态
    pub async fn check_pool_health(pool: &sqlx::Pool<sqlx::Sqlite>) -> AppResult<()> {
        let pool_size = pool.size();
        let idle_connections = pool.num_idle();
        let active_connections = pool_size.saturating_sub(idle_connections as u32);

        log::info!(
            "Connection pool status - Total: {}, Active: {}, Idle: {}",
            pool_size,
            active_connections,
            idle_connections
        );

        // 检查连接池健康状态
        if pool_size > 0 && active_connections > pool_size * 80 / 100 {
            log::warn!(
                "Connection pool usage is high: {}%",
                active_connections * 100 / pool_size
            );
        }

        Ok(())
    }
}

/// 事务建议器
/// 提供事务使用建议和最佳实践
pub struct TransactionAdvisor;

impl TransactionAdvisor {
    /// 建议使用事务的场景
    pub fn should_use_transaction(operation: &str) -> bool {
        // 判断是否需要事务的场景
        let transaction_required_patterns = [
            "create", "update", "delete", "insert", "transfer", "move", "modify", "change",
        ];

        transaction_required_patterns
            .iter()
            .any(|&pattern| operation.to_lowercase().contains(pattern))
    }

    /// 建议事务隔离级别
    pub fn recommend_isolation_level(operation_type: &str) -> &'static str {
        match operation_type {
            "read_only" | "select" | "query" => "READ UNCOMMITTED",
            "analytics" | "report" => "READ COMMITTED",
            "critical_write" | "financial" => "SERIALIZABLE",
            _ => "READ COMMITTED",
        }
    }

    /// 建议批量操作大小
    pub fn recommend_batch_size(operation: &str) -> usize {
        match operation {
            "insert" => 1000,
            "update" => 500,
            "delete" => 1000,
            _ => 100,
        }
    }
}

/// 事务装饰器
/// 为数据库操作提供简单的事务包装建议
pub struct TransactionDecorator;

impl TransactionDecorator {
    /// 执行简单事务的建议方法
    /// 注意：这是一个示例实现，实际使用时应该根据具体需求调整
    pub async fn execute_safely<F, T>(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        operation_name: &str,
        operation: F,
    ) -> AppResult<T>
    where
        F: FnOnce() -> futures::future::BoxFuture<'static, AppResult<T>>,
    {
        // 如果不需要事务，直接执行
        if !TransactionAdvisor::should_use_transaction(operation_name) {
            log::debug!("Executing non-transactional operation: {}", operation_name);
            return operation().await;
        }

        log::debug!("Executing transactional operation: {}", operation_name);

        // 这里应该实现实际的事务逻辑
        // 由于高阶类型复杂性，这里提供框架建议
        let context = TransactionUtils::create_transaction_context(operation_name, "execution");

        // 实际实现应该在具体的 repository 层完成
        operation()
            .await
            .map_err(|e| AppError::internal(format!("{}: {}", context, e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_config() {
        let config = TransactionConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 100);
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.enable_batch_optimization);
    }

    #[test]
    fn test_transaction_utils() {
        let context = TransactionUtils::create_transaction_context("create_room", "insert_record");
        assert_eq!(
            context,
            "Transaction failed during create_room - insert_record"
        );
    }

    #[test]
    fn test_retryable_error_detection() {
        let db_error = AppError::Database(sqlx::Error::PoolTimedOut);
        let auth_error = AppError::authentication("Invalid token");

        assert!(TransactionUtils::is_retryable_error(&db_error));
        assert!(!TransactionUtils::is_retryable_error(&auth_error));
    }

    #[test]
    fn test_transaction_advisor() {
        assert!(TransactionAdvisor::should_use_transaction("create_user"));
        assert!(TransactionAdvisor::should_use_transaction("update_profile"));
        assert!(!TransactionAdvisor::should_use_transaction("get_user_info"));

        assert_eq!(
            TransactionAdvisor::recommend_isolation_level("read_only"),
            "READ UNCOMMITTED"
        );
        assert_eq!(
            TransactionAdvisor::recommend_isolation_level("financial"),
            "SERIALIZABLE"
        );

        assert_eq!(TransactionAdvisor::recommend_batch_size("insert"), 1000);
        assert_eq!(TransactionAdvisor::recommend_batch_size("update"), 500);
    }
}
