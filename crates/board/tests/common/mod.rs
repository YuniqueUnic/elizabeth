/// 测试公共模块
///
/// 提供测试常用的工具和配置
pub mod fixtures;
pub mod http;
pub mod mocks;

use anyhow::Result;
use std::sync::Arc;

use board::config::{AppConfig, AuthConfig, RoomConfig, ServerConfig, StorageConfig};
use board::constants::{
    room::DEFAULT_MAX_ROOM_CONTENT_SIZE, room::DEFAULT_MAX_TIMES_ENTER_ROOM,
    upload::DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
};
use board::db::{DbPoolSettings, run_migrations};
use board::state::AppState;
use utoipa_axum::{router::OpenApiRouter, routes};

const TEST_DB_URL: &str = "sqlite::memory:";

/// 创建测试应用
pub async fn create_test_app() -> Result<(axum::Router, Arc<board::db::DbPool>)> {
    // 创建测试数据库
    let db_pool = Arc::new(
        DbPoolSettings::new(TEST_DB_URL)
            .with_max_connections(1)
            .with_min_connections(1)
            .create_pool()
            .await?,
    );

    // 运行迁移
    run_migrations(db_pool.as_ref(), TEST_DB_URL).await?;
    // 保底：确保关键表/列存在（覆盖潜在缺列）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS rooms (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            slug TEXT NOT NULL UNIQUE,
            password TEXT,
            status INTEGER NOT NULL DEFAULT 0,
            max_size INTEGER NOT NULL DEFAULT 10485760,
            current_size INTEGER NOT NULL DEFAULT 0,
            max_times_entered INTEGER NOT NULL DEFAULT 100,
            current_times_entered INTEGER NOT NULL DEFAULT 0,
            expire_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            permission INTEGER NOT NULL DEFAULT 1
        )
        "#,
    )
    .execute(db_pool.as_ref())
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS room_refresh_tokens (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            room_id INTEGER NOT NULL,
            access_token_jti TEXT NOT NULL,
            token_hash TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            last_used_at TEXT,
            is_revoked INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(db_pool.as_ref())
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS token_blacklist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            jti TEXT NOT NULL UNIQUE,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(db_pool.as_ref())
    .await?;

    // 创建测试配置
    let app_config = AppConfig {
        server: ServerConfig::default(),
        storage: StorageConfig {
            root: std::env::temp_dir(),
            upload_reservation_ttl_seconds: DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
        },
        room: RoomConfig {
            max_content_size: DEFAULT_MAX_ROOM_CONTENT_SIZE,
            max_times_entered: DEFAULT_MAX_TIMES_ENTER_ROOM,
        },
        auth: AuthConfig::new("test-secret-key-for-unit-testing-123456789".to_string())?,
    };

    // 创建应用状态
    let app_state = Arc::new(AppState::new(app_config, db_pool.clone())?);

    // 创建路由
    let (status_router, mut api) = board::route::status::api_router().split_for_parts();
    let (room_router, room_api) =
        board::route::room::api_router(app_state.clone()).split_for_parts();
    let (auth_router, auth_api) =
        board::route::auth::auth_router(app_state.clone()).split_for_parts();

    api.merge(room_api);
    api.merge(auth_api);

    let app = status_router.merge(room_router).merge(auth_router);

    Ok((app, db_pool))
}
