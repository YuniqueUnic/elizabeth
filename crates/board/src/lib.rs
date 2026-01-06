pub mod cmd;
pub mod config;
pub mod constants;
pub mod db;
pub mod errors;
mod handlers;
mod init;
pub mod middleware;
pub use board_protocol::dto;
pub use board_protocol::models;
pub mod permissions;
pub mod repository;
pub mod route;
pub mod services;
pub mod state;
pub mod storage;
pub mod validation;
pub mod websocket;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Duration;

use clap::Parser;
use shadow_rs::shadow;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::config::{AppConfig, AuthConfig, RoomConfig, ServerConfig, StorageConfig};
use crate::constants::{
    storage::DEFAULT_STORAGE_ROOT, upload::DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
};
use crate::db::{DbKind, DbPoolSettings, init_db, run_migrations};
use crate::init::{cfg_service, const_service, log_service};
use crate::repository::room_refresh_token_repository::{
    RoomRefreshTokenRepository, TokenBlacklistRepository,
};
use crate::services::{RoomTokenService, refresh_token_service::RefreshTokenService};
use crate::state::AppState;
use configrs::Config;
use sqlx::sqlite::SqliteJournalMode;

shadow!(build);

pub async fn run() -> anyhow::Result<()> {
    const_service::init();

    let cli = cmd::Cli::parse();
    println!("Parsed CLI arguments: {cli:?}");
    match cli {
        cmd::Cli::Start(args) => {
            let cfg = cfg_service::init(&args)?;
            start_server(&cfg).await?
        }
        #[cfg(feature = "completions")]
        cmd::Cli::Completions { shell } => cmd::output_completions(shell)?,
    }
    Ok(())
}

async fn start_server(cfg: &Config) -> anyhow::Result<()> {
    println!("Starting server with args: {cfg:#?}");
    log_service::init(cfg);

    // 初始化数据库
    let db_url = cfg.app.database.url.clone();
    let sqlite_journal = parse_sqlite_journal_mode(cfg.app.database.journal_mode.as_str());
    let db_settings = DbPoolSettings::new(db_url.clone())
        .with_max_connections(cfg.app.database.max_connections)
        .with_min_connections(cfg.app.database.min_connections);
    let db_pool = init_db(&db_settings).await?;
    if matches!(DbKind::detect(&db_url), DbKind::Sqlite) {
        apply_sqlite_journal_mode(&db_pool, sqlite_journal).await?;
    }
    run_migrations(&db_pool, &db_url).await?;
    let db_pool = Arc::new(db_pool);

    // 创建应用配置
    let app_config = AppConfig {
        server: ServerConfig {
            host: cfg.app.server.addr.clone(),
            port: cfg.app.server.port,
        },
        database: crate::config::DatabaseConfig {
            url: cfg.app.database.url.clone(),
            max_connections: cfg.app.database.max_connections.unwrap_or(10),
            min_connections: cfg.app.database.min_connections.unwrap_or(1),
            journal_mode: cfg.app.database.journal_mode.clone(),
        },
        storage: StorageConfig {
            root: if cfg.app.storage.root.trim().is_empty() {
                PathBuf::from(DEFAULT_STORAGE_ROOT)
            } else {
                PathBuf::from(&cfg.app.storage.root)
            },
            upload_reservation_ttl_seconds: if cfg.app.upload.reservation_ttl_seconds <= 0 {
                DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS
            } else {
                cfg.app.upload.reservation_ttl_seconds
            },
        },
        room: RoomConfig {
            max_content_size: cfg.app.room.max_size,
            max_times_entered: cfg.app.room.max_times_entered,
        },
        auth: AuthConfig::new(cfg.app.jwt.secret.clone())
            .map_err(|e| anyhow::anyhow!("Invalid JWT config: {}", e))?
            .with_ttl(cfg.app.jwt.ttl_seconds)
            .with_leeway(cfg.app.jwt.leeway_seconds),
    };

    // 创建应用状态
    let app_state = Arc::new(AppState::new(app_config, db_pool)?);

    let addr: SocketAddr = format!("{}:{}", cfg.app.server.addr, cfg.app.server.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let (scalar_path, router) = build_api_router(app_state, cfg);

    println!("Server listening on http://{addr}");
    println!("Scalar listening on http://{addr}{}", &scalar_path);

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(anyhow::Error::new)
}

fn parse_sqlite_journal_mode(value: &str) -> SqliteJournalMode {
    match value.to_ascii_lowercase().as_str() {
        "delete" => SqliteJournalMode::Delete,
        "truncate" => SqliteJournalMode::Truncate,
        "persist" => SqliteJournalMode::Persist,
        "memory" => SqliteJournalMode::Memory,
        "off" => SqliteJournalMode::Off,
        "wal" | "" => SqliteJournalMode::Wal,
        other => {
            log::warn!(
                "Unknown SQLite journal_mode '{}', fallback to WAL. Supported values: wal, delete, truncate, persist, memory, off.",
                other
            );
            SqliteJournalMode::Wal
        }
    }
}

async fn apply_sqlite_journal_mode(
    pool: &crate::db::DbPool,
    mode: SqliteJournalMode,
) -> anyhow::Result<()> {
    let pragma_value = match mode {
        SqliteJournalMode::Delete => "DELETE",
        SqliteJournalMode::Truncate => "TRUNCATE",
        SqliteJournalMode::Persist => "PERSIST",
        SqliteJournalMode::Memory => "MEMORY",
        SqliteJournalMode::Off => "OFF",
        SqliteJournalMode::Wal => "WAL",
    };
    let statement = format!("PRAGMA journal_mode = {pragma_value}");
    sqlx::query(&statement).execute(pool).await?;
    Ok(())
}

fn build_api_router(app_state: Arc<AppState>, cfg: &configrs::Config) -> (String, axum::Router) {
    let (status_router, mut api) = route::status::api_router().split_for_parts();
    let (room_router, room_api) = route::room::api_router(app_state.clone()).split_for_parts();
    let (auth_router, auth_api) = route::auth::auth_router(app_state.clone()).split_for_parts();

    let router = status_router.merge(room_router).merge(auth_router);
    api.merge(room_api);
    api.merge(auth_api);

    let (scalar, scalar_path) = route::scalar(api);
    let router = router.merge(scalar);

    // 集成 WebSocket 路由
    let ws_router = route::ws::api_router(app_state.clone());
    let router = router.merge(ws_router);

    // Apply middleware configuration
    let middleware_config = crate::middleware::from_app_config(cfg);
    let router = crate::middleware::apply(&middleware_config, router);

    (scalar_path, router)
}
