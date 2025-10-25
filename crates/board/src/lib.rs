pub mod cmd;
pub mod db;
pub mod errors;
mod handlers;
mod init;
pub mod models;
pub mod repository;
pub mod route;
pub mod services;
pub mod state;
pub mod transaction;
pub mod validation;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Duration;

use clap::Parser;
use shadow_rs::shadow;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::db::{DbPoolSettings, init_db, run_migrations};
use crate::handlers::content::{DEFAULT_STORAGE_ROOT, DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS};
use crate::init::{cfg_service, const_service, log_service};
use crate::repository::room_refresh_token_repository::{
    SqliteRoomRefreshTokenRepository, SqliteTokenBlacklistRepository,
};
use crate::services::{RoomTokenService, refresh_token_service::RefreshTokenService};
use crate::state::{AppState, RoomDefaults};
use configrs::Config;

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
    let db_settings = DbPoolSettings::new(cfg.app.database.url.clone())
        .with_max_connections(cfg.app.database.max_connections)
        .with_min_connections(cfg.app.database.min_connections);
    let db_pool = init_db(&db_settings).await?;
    run_migrations(&db_pool).await?;
    let db_pool = Arc::new(db_pool);

    let token_service = RoomTokenService::with_config(
        Arc::new(cfg.app.jwt.secret.clone()),
        cfg.app.jwt.ttl_seconds,
        cfg.app.jwt.leeway_seconds,
    );

    // 创建刷新令牌仓库
    let refresh_token_repo = Arc::new(SqliteRoomRefreshTokenRepository::new(db_pool.clone()));
    let blacklist_repo = Arc::new(SqliteTokenBlacklistRepository::new(db_pool.clone()));

    // 创建刷新令牌服务
    let refresh_token_service = RefreshTokenService::with_defaults(
        token_service.clone(),
        refresh_token_repo,
        blacklist_repo,
    );

    let room_defaults = RoomDefaults {
        max_size: cfg.app.room.max_size,
        max_times_entered: cfg.app.room.max_times_entered,
    };

    let upload_ttl_seconds = if cfg.app.upload.reservation_ttl_seconds <= 0 {
        DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS
    } else {
        cfg.app.upload.reservation_ttl_seconds
    };
    let storage_root = if cfg.app.storage.root.trim().is_empty() {
        PathBuf::from(DEFAULT_STORAGE_ROOT)
    } else {
        PathBuf::from(&cfg.app.storage.root)
    };

    let app_state = Arc::new(AppState::new(
        db_pool.clone(),
        storage_root,
        Duration::seconds(upload_ttl_seconds),
        room_defaults,
        token_service,
        refresh_token_service,
    ));

    let addr: SocketAddr = format!("{}:{}", cfg.app.server.addr, cfg.app.server.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let (scalar_path, router) = build_api_router(app_state);

    println!("Server listening on http://{addr}");
    println!("Scalar listening on http://{addr}{}", &scalar_path);

    axum::serve(listener, router.into_make_service())
        .await
        .map_err(anyhow::Error::new)
}

fn build_api_router(app_state: Arc<AppState>) -> (String, axum::Router) {
    let (root_router, mut api) = OpenApiRouter::new()
        .routes(routes!(route::openapi))
        .split_for_parts();
    let (status_router, status_api) = route::status::api_router().split_for_parts();
    let (room_router, room_api) = route::room::api_router(app_state.clone()).split_for_parts();
    let (auth_router, auth_api) = route::auth::auth_router(app_state).split_for_parts();

    let router = root_router
        .merge(status_router)
        .merge(room_router)
        .merge(auth_router);
    api.merge(status_api);
    api.merge(room_api);
    api.merge(auth_api);

    let (scalar, scalar_path) = route::scalar(api);
    let router = router.merge(scalar);
    (scalar_path, router)
}
