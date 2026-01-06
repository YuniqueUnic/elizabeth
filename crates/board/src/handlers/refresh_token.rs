use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::errors::{AppError, AppResult};
use crate::models::{RefreshTokenRequest, RefreshTokenResponse};
use crate::repository::room_repository::IRoomRepository;
use crate::services::refresh_token_service::RefreshTokenService;
use crate::state::AppState;

/// 刷新访问令牌
///
/// 使用刷新令牌获取新的访问令牌和刷新令牌对
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "令牌刷新成功", body = RefreshTokenResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "刷新令牌无效或已过期"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn refresh_token(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<RefreshTokenRequest>,
) -> AppResult<Json<RefreshTokenResponse>> {
    let refresh_service = app_state.refresh_token_service();

    let response = refresh_service
        .refresh_access_token(&request.refresh_token)
        .await
        .map_err(|e| {
            logrs::error!("Failed to refresh access token: {}", e);
            AppError::authentication("Invalid or expired refresh token")
        })?;

    Ok(Json(response))
}

/// 撤销令牌
///
/// 撤销指定的访问令牌及其关联的刷新令牌
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "authentication",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "令牌撤销成功"),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "令牌无效"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn revoke_token(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<LogoutRequest>,
) -> AppResult<String> {
    let refresh_service = app_state.refresh_token_service();

    // 首先验证令牌有效性
    let claims = app_state
        .token_service()
        .decode(&request.access_token)
        .map_err(|_| AppError::authentication("Invalid access token"))?;

    // 撤销令牌
    refresh_service
        .revoke_token(&claims.jti)
        .await
        .map_err(|e| {
            logrs::error!("Failed to revoke token: {}", e);
            AppError::internal("Failed to revoke token")
        })?;

    Ok("Token revoked successfully".to_string())
}

/// 登出请求结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    /// 访问令牌
    pub access_token: String,
}

/// 清理过期令牌
///
/// 清理过期的刷新令牌和黑名单记录（管理员功能）
#[utoipa::path(
    delete,
    path = "/api/v1/auth/cleanup",
    tag = "authentication",
    responses(
        (status = 200, description = "清理完成", body = CleanupResponse),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn cleanup_expired_tokens(
    State(app_state): State<Arc<AppState>>,
) -> AppResult<Json<CleanupResponse>> {
    let refresh_service = app_state.refresh_token_service();

    let cleaned_count = refresh_service.cleanup_expired().await.map_err(|e| {
        logrs::error!("Failed to cleanup expired tokens: {}", e);
        AppError::internal("Failed to cleanup expired tokens")
    })?;

    Ok(Json(CleanupResponse {
        cleaned_records: cleaned_count,
        message: "Cleanup completed successfully".to_string(),
    }))
}

/// 清理响应结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CleanupResponse {
    /// 清理的记录数量
    pub cleaned_records: u64,
    /// 操作结果消息
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DbPoolSettings, run_migrations};
    use crate::models::Room;
    use crate::repository::room_refresh_token_repository::{
        RoomRefreshTokenRepository, TokenBlacklistRepository,
    };
    use crate::services::token::RoomTokenService;
    use chrono::Duration;
    use std::sync::Arc;

    const TEST_DB_URL: &str = "sqlite::memory:";

    async fn create_test_app_state() -> Arc<AppState> {
        use crate::config::{AppConfig, AuthConfig, RoomConfig, ServerConfig, StorageConfig};
        use crate::constants::upload::DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS;
        use crate::constants::{
            room::DEFAULT_MAX_ROOM_CONTENT_SIZE, room::DEFAULT_MAX_TIMES_ENTER_ROOM,
            test::TEST_JWT_SECRET,
        };
        let settings = DbPoolSettings::new(TEST_DB_URL)
            .with_max_connections(1)
            .with_min_connections(1);
        let db_pool = Arc::new(settings.create_pool().await.unwrap());
        run_migrations(db_pool.as_ref(), TEST_DB_URL).await.unwrap();
        sqlx::query("DROP TABLE IF EXISTS rooms")
            .execute(db_pool.as_ref())
            .await
            .ok();
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
        .await
        .unwrap();
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
        .await
        .unwrap();
        sqlx::query("DROP TABLE IF EXISTS room_refresh_tokens")
            .execute(db_pool.as_ref())
            .await
            .ok();
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
        .await
        .unwrap();

        // 创建测试配置
        let app_config = AppConfig {
            server: ServerConfig::default(),
            database: crate::config::DatabaseConfig::default(),
            storage: StorageConfig {
                root: std::env::temp_dir(),
                upload_reservation_ttl_seconds: DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
            },
            room: RoomConfig {
                max_content_size: DEFAULT_MAX_ROOM_CONTENT_SIZE,
                max_times_entered: DEFAULT_MAX_TIMES_ENTER_ROOM,
            },
            auth: AuthConfig::new("test-secret-key-for-unit-testing-123456789".to_string())
                .unwrap(),
        };

        Arc::new(AppState::new(app_config, db_pool).unwrap())
    }

    #[tokio::test]
    async fn test_refresh_token_handler() {
        let app_state = create_test_app_state().await;

        // 在数据库中创建测试房间
        let room = Room::new("test_room".to_string(), Some("password".to_string()));
        let room_with_id = app_state
            .services()
            .room_repository
            .create(&room)
            .await
            .unwrap();

        // 签发初始令牌对
        let initial_response = app_state
            .refresh_token_service()
            .issue_token_pair(&room_with_id)
            .await
            .unwrap();

        // 测试刷新令牌
        let refresh_request = RefreshTokenRequest {
            refresh_token: initial_response.refresh_token,
        };

        let result = refresh_token(State(app_state.clone()), Json(refresh_request)).await;

        assert!(result.is_ok());
        let refreshed_response = result.unwrap().0;
        assert!(!refreshed_response.access_token.is_empty());
        assert!(!refreshed_response.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_revoke_token_handler() {
        let app_state = create_test_app_state().await;

        // 在数据库中创建测试房间
        let room = Room::new("test_room".to_string(), Some("password".to_string()));
        let room_with_id = app_state
            .services()
            .room_repository
            .create(&room)
            .await
            .unwrap();

        // 签发令牌对
        let response = app_state
            .refresh_token_service()
            .issue_token_pair(&room_with_id)
            .await
            .unwrap();

        // 测试撤销令牌
        let logout_request = LogoutRequest {
            access_token: response.access_token,
        };

        let result = revoke_token(State(app_state.clone()), Json(logout_request)).await;

        assert!(result.is_ok());
    }
}
