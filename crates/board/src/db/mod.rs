use anyhow::Result;
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::str::FromStr;
use tracing::{error, info};

/// 数据库连接池
pub type DbPool = Pool<Sqlite>;

/// 初始化数据库连接池
pub async fn init_db(database_url: &str) -> Result<DbPool> {
    info!("初始化数据库连接池：{}", database_url);

    let connect_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(30));

    let pool = SqlitePoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
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

#[allow(unused)]
/// 数据库连接配置
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:./app.db".to_string(),
            max_connections: 20,
            min_connections: 5,
        }
    }
}

#[allow(unused)]
impl DatabaseConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn with_max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = max_connections;
        self
    }

    pub fn with_min_connections(mut self, min_connections: u32) -> Self {
        self.min_connections = min_connections;
        self
    }

    pub async fn create_pool(&self) -> Result<DbPool> {
        init_db(&self.url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_init() {
        let config = DatabaseConfig::new("sqlite::memory:");
        let pool = config.create_pool().await;
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn test_migrations() {
        let config = DatabaseConfig::new("sqlite::memory:");
        let pool = config.create_pool().await.unwrap();
        let result = run_migrations(&pool).await;
        assert!(result.is_ok());
    }
}
