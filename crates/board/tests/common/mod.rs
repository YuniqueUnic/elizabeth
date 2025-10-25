/// 测试公共模块
///
/// 提供测试常用的工具和配置
pub mod fixtures;
pub mod http;
pub mod mocks;

use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::Arc;

use board::config::{AppConfig, AuthConfig, RoomConfig, ServerConfig, StorageConfig};
use board::constants::{
    room::DEFAULT_MAX_ROOM_CONTENT_SIZE, room::DEFAULT_MAX_TIMES_ENTER_ROOM,
    upload::DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
};
use board::db::{DbPoolSettings, init_db};
use board::state::AppState;
use utoipa_axum::{router::OpenApiRouter, routes};

/// 创建测试应用
pub async fn create_test_app() -> Result<(axum::Router, Arc<SqlitePool>)> {
    // 创建测试数据库
    let db_settings = DbPoolSettings::new("sqlite::memory:");
    let db_pool = Arc::new(init_db(&db_settings).await?);

    // 运行迁移
    sqlx::migrate!("./migrations").run(&*db_pool).await?;

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
    let (root_router, api) = OpenApiRouter::new()
        .routes(routes!(board::route::openapi))
        .split_for_parts();
    let (status_router, status_api) = board::route::status::api_router().split_for_parts();
    let (room_router, room_api) =
        board::route::room::api_router(app_state.clone()).split_for_parts();
    let (auth_router, auth_api) =
        board::route::auth::auth_router(app_state.clone()).split_for_parts();
    let app = root_router
        .merge(status_router)
        .merge(room_router)
        .merge(auth_router);

    Ok((app, db_pool))
}
