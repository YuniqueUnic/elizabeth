#![allow(unused_imports, unused_variables, dead_code)]
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
mod scheduler;
pub mod services;
pub mod state;
pub mod storage;
mod tasks;
pub mod validation;
pub mod websocket;

#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Duration;
use rust_embed::RustEmbed;

use clap::Parser;
use shadow_rs::shadow;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::config::{AppConfig, AuthConfig, RoomConfig, ServerConfig, StorageConfig};
use crate::constants::{
    storage::DEFAULT_STORAGE_ROOT, upload::DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
};
use crate::db::{
    DbKind, DbPoolSettings, backup_sqlite_before_migrations, has_pending_sqlite_migrations,
    init_db, run_migrations,
};
use crate::init::{cfg_service, const_service, log_service};
use crate::repository::RoomUploadReservationRepository;
use crate::repository::room_refresh_token_repository::{
    RoomRefreshTokenRepository, TokenBlacklistRepository,
};
use crate::scheduler::{TaskRegistration, TaskScheduler};
use crate::services::{RoomTokenService, refresh_token_service::RefreshTokenService};
use crate::state::AppState;
use crate::tasks::{RoomLifecycleTask, TokenCleanupTask, UploadCleanupTask};
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
        if has_pending_sqlite_migrations(&db_pool).await? {
            backup_sqlite_before_migrations(&db_pool, &db_url).await?;
        }
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
        room: RoomConfig::try_from(&cfg.app.room)?,
        auth: AuthConfig::new(cfg.app.jwt.secret.clone())
            .map_err(|e| anyhow::anyhow!("Invalid JWT config: {}", e))?
            .with_ttl(cfg.app.jwt.ttl_seconds)
            .with_leeway(cfg.app.jwt.leeway_seconds)
            .with_refresh_policy(
                cfg.app.jwt.refresh_ttl_seconds,
                cfg.app.jwt.cleanup_interval_seconds,
                cfg.app.jwt.enable_refresh_token_rotation,
            ),
    };

    // 创建应用状态
    let app_state = Arc::new(AppState::new(app_config, db_pool)?);
    let migrated_passwords = crate::services::migrate_legacy_room_passwords(
        &app_state.db_pool,
        app_state.room_password_service(),
    )
    .await?;
    if migrated_passwords > 0 {
        log::info!(
            "Migrated {} legacy room passwords to Argon2id",
            migrated_passwords
        );
    }

    let scheduler = start_scheduler(app_state.clone(), cfg);
    let scheduler_cancellation = scheduler.cancellation_token();

    let addr: SocketAddr = format!("{}:{}", cfg.app.server.addr, cfg.app.server.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_addr = listener.local_addr()?;

    let (scalar_path, router) = build_api_router(app_state, cfg);

    println!("Server listening on http://{actual_addr}");
    println!("Scalar listening on http://{actual_addr}{}", &scalar_path);

    let result = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(scheduler_cancellation))
    .await
    .map_err(anyhow::Error::new);
    scheduler.shutdown().await;
    result
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
    let (admin_router, admin_api) = route::admin::api_router(app_state.clone()).split_for_parts();
    let (config_router, config_api) =
        route::config::api_router(app_state.clone()).split_for_parts();

    let router = status_router
        .merge(room_router)
        .merge(auth_router)
        .merge(admin_router)
        .merge(config_router);
    api.merge(room_api);
    api.merge(auth_api);
    api.merge(admin_api);
    api.merge(config_api);

    // Expose a machine-consumable OpenAPI document for client generation.
    // Keep it in parallel with the JSON Schema exported via build.rs.
    let openapi_path = format!("{}/openapi.json", route::API_PREFIX);
    let openapi_json = serde_json::to_string_pretty(&api).unwrap_or_else(|e| {
        log::error!("Failed to serialize OpenAPI: {e}");
        "{}".to_string()
    });
    let router = router.route(
        &openapi_path,
        axum::routing::get({
            let openapi_json = openapi_json.clone();
            move || async move {
                (
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    openapi_json.clone(),
                )
            }
        }),
    );

    let (scalar, scalar_path) = route::scalar(api);
    let router = router.merge(scalar);

    // 集成 WebSocket 路由
    let ws_router = route::ws::api_router(app_state.clone());
    let router = router.merge(ws_router);

    // Apply middleware configuration
    let middleware_config = crate::middleware::from_app_config(cfg);
    let router = crate::middleware::apply(&middleware_config, router);

    // SPA fallback：把嵌入的前端静态资源挂到最低优先级
    // - 精确匹配静态文件（_next/*, favicon.ico 等）→ 直接返回
    // - /api/* 路径 → JSON 404（防止穿透到 SPA）
    // - 所有其他路径 → 返回 index.html（由客户端 React Router 接管）
    let router = router.fallback(spa_fallback);

    (scalar_path, router)
}

/// Next.js `output: 'export'` 产物目录，由 rust-embed 在编译期打包进二进制
/// 路径在 crate 内部，确保 `cargo publish` 能正确打包。
/// CI / Docker / 本地构建时需先将 `web/out` 复制到此目录。
#[derive(RustEmbed)]
#[folder = "web-out"]
#[exclude = "*.map"] // 排除 source map，减小体积
struct EmbeddedSpa;

/// SPA Fallback Handler（极简三步逻辑）
async fn spa_fallback(request: Request<Body>) -> Response {
    let path = request.uri().path().trim_start_matches('/').to_string();

    // Step 1：防御拦截 API 路由 — /api/* 绝不降级为 HTML
    if path.starts_with("api/") {
        return (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            Body::from(r#"{"error":"Not Found","code":404}"#),
        )
            .into_response();
    }

    // Step 2：精确匹配嵌入的静态资产（_next/*, *.css, *.js, favicon.ico 等）
    if let Some(asset) = EmbeddedSpa::get(&path) {
        let mime = mime_guess::from_path(&path)
            .first_or_octet_stream()
            .to_string();
        return Response::builder()
            .status(StatusCode::OK)
            .header(axum::http::header::CONTENT_TYPE, &mime)
            .body(Body::from(asset.data))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    // Step 3：SPA 智能降级
    // - 空路径（根路由 /）→ 首页 index.html
    // - 含子路径的路由（/room-abc, /my-room 等）→ 房间模板 _.html
    //   （Next.js 静态导出用占位符 "_" 预生成的房间页模板）
    //   客户端 usePathname() 从真实浏览器 URL 读取实际房间名，无水合冲突
    let fallback_file = if path.is_empty() {
        "index.html"
    } else {
        "_.html"
    };

    serve_html_asset(fallback_file)
}

fn serve_html_asset(name: &str) -> Response {
    match EmbeddedSpa::get(name) {
        Some(html) => Response::builder()
            .status(StatusCode::OK)
            .header(
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            )
            .header(
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("no-store, max-age=0"),
            )
            .body(Body::from(html.data))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "Frontend asset '{}' not embedded. Run `bun run build:embedded` in web/ first, or rebuild `elizabeth-board` without `ELIZABETH_SKIP_WEB_BUILD`.",
                name
            ),
        )
            .into_response(),
    }
}

fn start_scheduler(app_state: Arc<AppState>, cfg: &Config) -> scheduler::SchedulerHandle {
    let room_interval = std::time::Duration::from_secs(cfg.app.gc.interval_seconds.max(5));
    let auth_interval =
        std::time::Duration::from_secs(app_state.config.auth.cleanup_interval_seconds.max(5));
    let timeout = std::time::Duration::from_secs(300);
    let upload_repository = Arc::new(RoomUploadReservationRepository::new(
        app_state.db_pool.clone(),
    ));

    TaskScheduler::new(vec![
        TaskRegistration {
            interval: room_interval,
            timeout,
            task: Arc::new(RoomLifecycleTask::new(
                app_state.services.room_lifecycle.clone(),
                app_state.connection_manager.clone(),
                cfg.app.gc.batch_limit.clamp(1, 10_000),
                app_state.config.room.share_disabled_lock_duration,
            )),
        },
        TaskRegistration {
            interval: auth_interval,
            timeout,
            task: Arc::new(TokenCleanupTask::new(
                app_state.services.refresh_token_service.clone(),
            )),
        },
        TaskRegistration {
            interval: room_interval,
            timeout,
            task: Arc::new(UploadCleanupTask::new(upload_repository)),
        },
    ])
    .start()
}

async fn shutdown_signal(cancellation: tokio_util::sync::CancellationToken) {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            log::warn!("Failed to install Ctrl+C handler: {error}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => log::warn!("Failed to install SIGTERM handler: {error}"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    cancellation.cancel();
}
