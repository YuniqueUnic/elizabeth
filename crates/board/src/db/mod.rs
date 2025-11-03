use anyhow::Result;
use logrs::{error, info};
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::str::FromStr;

use crate::constants::database::{
    ACQUIRE_TIMEOUT_SECS, BUSY_TIMEOUT_SECS, DEFAULT_DB_URL, DEFAULT_MAX_CONNECTIONS,
    DEFAULT_MIN_CONNECTIONS, IDLE_TIMEOUT_SECS, MAX_LIFETIME_SECS,
};

/// 数据库连接池
pub type DbPool = Pool<Sqlite>;

/// 初始化数据库连接池
pub async fn init_db(settings: &DbPoolSettings) -> Result<DbPool> {
    info!("初始化数据库连接池：{}", settings.url);

    let connect_options = SqliteConnectOptions::from_str(&settings.url)?
        .create_if_missing(true)
        .journal_mode(settings.journal_mode)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(BUSY_TIMEOUT_SECS));

    let (max_connections, min_connections) = settings.resolve_connection_limits();

    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(std::time::Duration::from_secs(ACQUIRE_TIMEOUT_SECS))
        .idle_timeout(std::time::Duration::from_secs(IDLE_TIMEOUT_SECS))
        .max_lifetime(std::time::Duration::from_secs(MAX_LIFETIME_SECS))
        .connect_with(connect_options)
        .await?;

    info!("数据库连接池初始化成功");
    Ok(pool)
}

/// 运行数据库迁移
pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    info!("开始运行数据库迁移");

    match sqlx::migrate!("./migrations").run(pool).await {
        Ok(_) => {
            info!("数据库迁移完成");
            Ok(())
        }
        Err(e) => {
            error!("数据库迁移失败：{}", e);
            Err(anyhow::anyhow!("数据库迁移失败：{}", e))
        }
    }
}

/// 数据库连接配置
#[derive(Debug, Clone)]
pub struct DbPoolSettings {
    pub url: String,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub journal_mode: SqliteJournalMode,
}

impl Default for DbPoolSettings {
    fn default() -> Self {
        Self {
            url: DEFAULT_DB_URL.to_string(),
            max_connections: Some(DEFAULT_MAX_CONNECTIONS),
            min_connections: Some(DEFAULT_MIN_CONNECTIONS),
            journal_mode: SqliteJournalMode::Wal,
        }
    }
}

impl DbPoolSettings {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn with_max_connections(mut self, max_connections: impl Into<Option<u32>>) -> Self {
        self.max_connections = max_connections.into();
        self
    }

    pub fn with_min_connections(mut self, min_connections: impl Into<Option<u32>>) -> Self {
        self.min_connections = min_connections.into();
        self
    }

    pub fn with_journal_mode(mut self, journal_mode: SqliteJournalMode) -> Self {
        self.journal_mode = journal_mode;
        self
    }

    pub fn resolve_connection_limits(&self) -> (u32, u32) {
        let max = self
            .max_connections
            .unwrap_or(DEFAULT_MAX_CONNECTIONS)
            .max(1);
        let desired_min = self.min_connections.unwrap_or(DEFAULT_MIN_CONNECTIONS);
        let min = desired_min.min(max);
        if desired_min > max {
            info!(
                "数据库连接池最小连接数 {} 大于最大连接数 {}，已自动调整为 {}",
                desired_min, max, min
            );
        }
        (max, min)
    }

    pub async fn create_pool(&self) -> Result<DbPool> {
        init_db(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_init() {
        let config = DbPoolSettings::new("sqlite::memory:");
        let pool = config.create_pool().await;
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn test_migrations() {
        let config = DbPoolSettings::new("sqlite::memory:");
        let pool = config.create_pool().await.unwrap();
        let result = run_migrations(&pool).await;
        assert!(result.is_ok());
    }
}
