use anyhow::Result;
use logrs::{error, info};
use sqlx::{Any, AnyPool, Executor, any::AnyPoolOptions};

use crate::constants::database::{
    ACQUIRE_TIMEOUT_SECS, DEFAULT_DB_URL, DEFAULT_MAX_CONNECTIONS, DEFAULT_MIN_CONNECTIONS,
    IDLE_TIMEOUT_SECS, MAX_LIFETIME_SECS,
};

/// 数据库连接池（统一支持 Sqlite / Postgres）
pub type DbPool = AnyPool;

/// 支持的数据库类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbKind {
    Sqlite,
    Postgres,
}

impl DbKind {
    pub fn detect(url: &str) -> Self {
        if url.starts_with("postgres") {
            DbKind::Postgres
        } else {
            DbKind::Sqlite
        }
    }
}

/// 初始化数据库连接池
pub async fn init_db(settings: &DbPoolSettings) -> Result<DbPool> {
    info!("初始化数据库连接池：{}", settings.url);

    let (max_connections, min_connections) = settings.resolve_connection_limits();

    // Ensure Any drivers (SQLite/Postgres) are registered before connecting.
    sqlx::any::install_default_drivers();

    let pool = AnyPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(std::time::Duration::from_secs(ACQUIRE_TIMEOUT_SECS))
        .idle_timeout(std::time::Duration::from_secs(IDLE_TIMEOUT_SECS))
        .max_lifetime(std::time::Duration::from_secs(MAX_LIFETIME_SECS))
        .connect(&settings.url)
        .await?;

    info!("数据库连接池初始化成功");
    Ok(pool)
}

/// 运行数据库迁移（根据 DbKind 选择目录）
pub async fn run_migrations(pool: &DbPool, url: &str) -> Result<()> {
    info!("开始运行数据库迁移");
    let kind = DbKind::detect(url);
    let primary = match kind {
        DbKind::Sqlite => std::path::Path::new("./migrations"),
        DbKind::Postgres => std::path::Path::new("./migrations_pg"),
    };
    // 兼容工作区根目录执行测试：回退到 crate 内的迁移目录
    let fallback = match kind {
        DbKind::Sqlite => std::path::Path::new("./crates/board/migrations"),
        DbKind::Postgres => std::path::Path::new("./crates/board/migrations_pg"),
    };
    let path = if primary.exists() { primary } else { fallback };

    match sqlx::migrate::Migrator::new(path).await?.run(pool).await {
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
}

impl Default for DbPoolSettings {
    fn default() -> Self {
        Self {
            url: DEFAULT_DB_URL.to_string(),
            max_connections: Some(DEFAULT_MAX_CONNECTIONS),
            min_connections: Some(DEFAULT_MIN_CONNECTIONS),
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
    async fn test_db_init_sqlite() {
        let config = DbPoolSettings::new("sqlite::memory:");
        let pool = config.create_pool().await;
        assert!(pool.is_ok());
    }
}
