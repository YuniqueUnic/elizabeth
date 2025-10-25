use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum_responses::http::HttpResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
) -> Result<Json<RefreshTokenResponse>, HttpResponse> {
    let refresh_service = app_state.refresh_token_service();

    let response = refresh_service
        .refresh_access_token(&request.refresh_token)
        .await
        .map_err(|e| {
            logrs::error!("Failed to refresh access token: {}", e);
            HttpResponse::Unauthorized().message("Invalid or expired refresh token")
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
) -> Result<HttpResponse, HttpResponse> {
    let refresh_service = app_state.refresh_token_service();

    // 首先验证令牌有效性
    let claims = app_state
        .token_service()
        .decode(&request.access_token)
        .map_err(|_| HttpResponse::Unauthorized().message("Invalid access token"))?;

    // 撤销令牌
    refresh_service
        .revoke_token(&claims.jti)
        .await
        .map_err(|e| {
            logrs::error!("Failed to revoke token: {}", e);
            HttpResponse::InternalServerError().message("Failed to revoke token")
        })?;

    Ok(HttpResponse::Ok().message("Token revoked successfully"))
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
) -> Result<Json<CleanupResponse>, HttpResponse> {
    let refresh_service = app_state.refresh_token_service();

    let cleaned_count = refresh_service.cleanup_expired().await.map_err(|e| {
        logrs::error!("Failed to cleanup expired tokens: {}", e);
        HttpResponse::InternalServerError().message("Failed to cleanup expired tokens")
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
    use crate::models::Room;
    use crate::repository::room_refresh_token_repository::{
        SqliteRoomRefreshTokenRepository, SqliteTokenBlacklistRepository,
    };
    use crate::services::token::RoomTokenService;
    use chrono::Duration;
    use sqlx::SqlitePool;
    use std::sync::Arc;

    async fn create_test_app_state() -> Arc<AppState> {
        use crate::config::{AppConfig, AuthConfig, RoomConfig, ServerConfig, StorageConfig};
        use crate::constants::upload::DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS;
        use crate::constants::{
            room::DEFAULT_MAX_ROOM_CONTENT_SIZE, room::DEFAULT_MAX_TIMES_ENTER_ROOM,
            test::TEST_JWT_SECRET,
        };
        use crate::db::init_db;

        let db_pool = Arc::new(
            init_db(&crate::db::DbPoolSettings::new("sqlite::memory:"))
                .await
                .unwrap(),
        );

        // 运行迁移
        sqlx::migrate!("./migrations").run(&*db_pool).await.unwrap();

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
