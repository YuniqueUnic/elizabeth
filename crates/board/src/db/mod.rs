use anyhow::Result;
use logrs::{error, info};
use sqlx::{Any, AnyPool, Executor, any::AnyPoolOptions};
use std::path::{Path, PathBuf};

use crate::constants::database::{
    ACQUIRE_TIMEOUT_SECS, BUSY_TIMEOUT_SECS, DEFAULT_DB_URL, DEFAULT_MAX_CONNECTIONS,
    DEFAULT_MIN_CONNECTIONS, IDLE_TIMEOUT_SECS, MAX_LIFETIME_SECS,
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

    let mut pool_options = AnyPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(std::time::Duration::from_secs(ACQUIRE_TIMEOUT_SECS))
        .idle_timeout(std::time::Duration::from_secs(IDLE_TIMEOUT_SECS))
        .max_lifetime(std::time::Duration::from_secs(MAX_LIFETIME_SECS));

    if DbKind::detect(&settings.url) == DbKind::Sqlite {
        let busy_timeout_ms = BUSY_TIMEOUT_SECS * 1_000;
        pool_options = pool_options.after_connect(move |connection, _metadata| {
            Box::pin(async move {
                let statement = format!("PRAGMA busy_timeout = {busy_timeout_ms}");
                connection.execute(statement.as_str()).await?;
                connection.execute("PRAGMA foreign_keys = ON").await?;
                Ok(())
            })
        });
    }

    let pool = pool_options.connect(&settings.url).await?;

    info!("数据库连接池初始化成功");
    Ok(pool)
}

/// 运行数据库迁移（根据 DbKind 选择目录）
pub async fn run_migrations(pool: &DbPool, url: &str) -> Result<()> {
    info!("开始运行数据库迁移");
    let kind = DbKind::detect(url);
    let path = migration_path(kind);

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

pub async fn has_pending_sqlite_migrations(pool: &DbPool) -> Result<bool> {
    let migrator = sqlx::migrate::Migrator::new(migration_path(DbKind::Sqlite)).await?;
    let table_exists: i64 = sqlx::query_scalar(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations') THEN 1 ELSE 0 END",
    )
    .fetch_one(pool)
    .await?;
    if table_exists == 0 {
        return Ok(migrator.iter().next().is_some());
    }

    let applied: Vec<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations")
        .fetch_all(pool)
        .await?;
    Ok(migrator
        .iter()
        .any(|migration| !applied.contains(&migration.version)))
}

fn migration_path(kind: DbKind) -> &'static Path {
    let primary = match kind {
        DbKind::Sqlite => Path::new("./migrations"),
        DbKind::Postgres => Path::new("./migrations_pg"),
    };
    let fallback = match kind {
        DbKind::Sqlite => Path::new("./crates/board/migrations"),
        DbKind::Postgres => Path::new("./crates/board/migrations_pg"),
    };
    if primary.exists() { primary } else { fallback }
}

pub async fn backup_sqlite_before_migrations(pool: &DbPool, url: &str) -> Result<Option<PathBuf>> {
    if DbKind::detect(url) != DbKind::Sqlite || url.contains(":memory:") {
        return Ok(None);
    }

    let user_table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_one(pool)
    .await?;
    if user_table_count == 0 {
        return Ok(None);
    }

    let database_path = sqlite_path_from_url(url)?;
    let parent = database_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = database_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("SQLite database URL has no file name"))?;
    let prefix = format!("{file_name}.bkp");
    let mut backups = existing_backups(parent, &prefix)?;
    let next = backups.last().map_or(1, |(iteration, _)| iteration + 1);
    let backup_path = parent.join(format!("{prefix}{next}"));
    let escaped = backup_path.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{escaped}'"))
        .execute(pool)
        .await?;

    backups.push((next, backup_path.clone()));
    backups.sort_by_key(|(iteration, _)| *iteration);
    let remove_count = backups.len().saturating_sub(3);
    for (_, path) in backups.into_iter().take(remove_count) {
        std::fs::remove_file(path)?;
    }

    info!("SQLite migration backup created: {}", backup_path.display());
    Ok(Some(backup_path))
}

fn sqlite_path_from_url(url: &str) -> Result<PathBuf> {
    let raw = url
        .strip_prefix("sqlite://")
        .ok_or_else(|| anyhow::anyhow!("Unsupported SQLite URL: {url}"))?
        .split('?')
        .next()
        .unwrap_or_default();
    if raw.is_empty() {
        return Err(anyhow::anyhow!("SQLite URL has no database path"));
    }
    Ok(PathBuf::from(raw))
}

fn existing_backups(directory: &Path, prefix: &str) -> Result<Vec<(u64, PathBuf)>> {
    let mut backups = Vec::new();
    if !directory.exists() {
        return Ok(backups);
    }
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(iteration) = name
            .strip_prefix(prefix)
            .and_then(|value| value.parse::<u64>().ok())
        {
            backups.push((iteration, entry.path()));
        }
    }
    backups.sort_by_key(|(iteration, _)| *iteration);
    Ok(backups)
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
