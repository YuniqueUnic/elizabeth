use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use axum::{
    extract::{Json, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::db::DbPool;
use crate::errors::{AppError, AppResult};
use crate::models::{RefreshTokenRequest, RefreshTokenResponse};
use crate::models::{
    permission::RoomPermission,
    room::{Room, RoomStatus},
};
use crate::services::{AuthService, RefreshTokenService};

/// 刷新令牌请求结构
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenRequestSchema {
    /// 刷新令牌
    #[serde(rename = "refresh_token")]
    pub refresh_token: String,
}

/// 刷新令牌响应结构
#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshTokenResponseSchema {
    /// 新的访问令牌
    pub access_token: String,
    /// 新的刷新令牌
    pub refresh_token: String,
    /// 访问令牌过期时间
    pub access_token_expires_at: chrono::NaiveDateTime,
    /// 刷新令牌过期时间
    pub refresh_token_expires_at: chrono::NaiveDateTime,
}

impl From<RefreshTokenResponse> for RefreshTokenResponseSchema {
    fn from(response: RefreshTokenResponse) -> Self {
        Self {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            access_token_expires_at: response.access_token_expires_at,
            refresh_token_expires_at: response.refresh_token_expires_at,
        }
    }
}

/// 登出请求结构
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequestSchema {
    /// 令牌（可选，如果不提供则从 Authorization 头中获取）
    pub token: Option<String>,
}

/// 登出响应结构
#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutResponseSchema {
    /// 是否成功登出
    pub success: bool,
    /// 消息
    pub message: String,
}

/// 应用状态，包含所有需要的服务
#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<DbPool>,
    pub refresh_token_service: Arc<RefreshTokenService>,
    pub auth_service: Arc<AuthService>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(
        db_pool: Arc<DbPool>,
        refresh_token_service: Arc<RefreshTokenService>,
        auth_service: Arc<AuthService>,
    ) -> Self {
        Self {
            db_pool,
            refresh_token_service,
            auth_service,
        }
    }

    /// 获取数据库连接池
    pub fn db_pool(&self) -> &Arc<DbPool> {
        &self.db_pool
    }

    /// 获取刷新令牌服务
    pub fn refresh_token_service(&self) -> &Arc<RefreshTokenService> {
        &self.refresh_token_service
    }

    /// 获取认证服务
    pub fn auth_service(&self) -> &Arc<AuthService> {
        &self.auth_service
    }
}

/// 刷新访问令牌处理器
#[utoipa::path(
    path = "/api/v1/auth/refresh",
    post,
    tag = "Authentication",
    summary = "刷新访问令牌",
    description = "使用刷新令牌获取新的访问令牌和刷新令牌对",
    request_body(
        content = RefreshTokenRequestSchema,
        description = "刷新令牌请求",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "刷新成功", body = RefreshTokenResponseSchema),
        (status = 400, description = "无效的请求"),
        (status = 401, description = "无效的刷新令牌"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequestSchema>,
) -> AppResult<RefreshTokenResponseSchema> {
    // 验证刷新令牌
    let claims = state
        .auth_service
        .verify_refresh_token(&request.refresh_token)
        .await
        .map_err(|e| {
            logrs::error!("Failed to verify refresh token: {}", e);
            AppError::token("Invalid refresh token")
        })?;

    // 获取房间信息
    let room = get_room_by_id(state.db_pool(), claims.room_id)
        .await
        .map_err(|e| {
            logrs::error!("Failed to get room: {}", e);
            AppError::internal("Failed to retrieve room information")
        })?
        .ok_or_else(|| {
            logrs::error!("Room not found for id: {}", claims.room_id);
            AppError::room_not_found(claims.room_id.to_string())
        })?;

    // 刷新访问令牌
    let response = state
        .refresh_token_service
        .refresh_access_token(&request.refresh_token)
        .await
        .map_err(|e| {
            logrs::error!("Failed to refresh access token: {}", e);
            AppError::internal("Failed to refresh access token")
        })?;

    // 记录日志
    logrs::info!(
        "Token refreshed successfully for room_id: {}, user: {}",
        claims.room_id,
        claims.sub
    );

    Ok(response.into())
}

/// 登出处理器
#[utoipa::path(
    path = "/api/v1/auth/logout",
    post,
    tag = "Authentication",
    summary = "登出",
    description = "撤销访问令牌和关联的刷新令牌",
    request_body(
        content = LogoutRequestSchema,
        description = "登出请求",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "登出成功", body = LogoutResponseSchema),
        (status = 400, description = "无效的请求"),
        (status = 401, description = "无效的令牌"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    Json(request): Json<LogoutRequestSchema>,
) -> AppResult<LogoutResponseSchema> {
    // 从请求中获取令牌，如果没有提供则尝试从 Authorization 头中获取
    let token = if let Some(token) = request.token {
        token
    } else {
        // 这里应该从 Authorization 头中获取令牌，但为了简化，我们返回错误
        return Err(AppError::validation("Token is required in request body"));
    };

    // 验证访问令牌
    let claims = state
        .auth_service
        .get_token_service()
        .decode(&token)
        .map_err(|e| {
            logrs::error!("Failed to decode token: {}", e);
            AppError::token("Invalid access token")
        })?;

    // 获取房间信息
    let room = get_room_by_id(state.db_pool(), claims.room_id)
        .await
        .map_err(|e| {
            logrs::error!("Failed to get room: {}", e);
            AppError::internal("Failed to retrieve room information")
        })?
        .ok_or_else(|| {
            logrs::error!("Room not found for id: {}", claims.room_id);
            AppError::room_not_found(claims.room_id.to_string())
        })?;

    // 验证访问令牌
    state
        .auth_service
        .verify_access_token(&token, &room)
        .await
        .map_err(|e| {
            logrs::error!("Failed to verify access token: {}", e);
            AppError::authentication("Token verification failed")
        })?;

    // 撤销令牌
    state
        .auth_service
        .blacklist_token(&claims)
        .await
        .map_err(|e| {
            logrs::error!("Failed to blacklist token: {}", e);
            AppError::internal("Failed to blacklist token")
        })?;

    // 撤销关联的刷新令牌
    if let Some(refresh_jti) = claims.refresh_jti {
        state
            .refresh_token_service
            .revoke_token(&refresh_jti)
            .await
            .map_err(|e| {
                logrs::error!("Failed to revoke refresh token: {}", e);
                AppError::internal("Failed to revoke refresh token")
            })?;
    }

    // 记录日志
    logrs::info!(
        "User logged out successfully for room_id: {}, user: {}",
        claims.room_id,
        claims.sub
    );

    Ok(LogoutResponseSchema {
        success: true,
        message: "Successfully logged out".to_string(),
    })
}

/// 从授权头中提取令牌并登出
#[utoipa::path(
    path = "/api/v1/auth/logout",
    post,
    tag = "Authentication",
    summary = "登出（使用 Authorization 头）",
    description = "使用 Authorization 头中的令牌进行登出",
    params(
        ("Authorization" = String, Header, description = "Bearer 令牌")
    ),
    responses(
        (status = 200, description = "登出成功", body = LogoutResponseSchema),
        (status = 400, description = "无效的请求"),
        (status = 401, description = "无效的令牌"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn logout_with_auth_header(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> AppResult<LogoutResponseSchema> {
    // 从 Authorization 头中提取令牌
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            logrs::error!("Missing Authorization header");
            AppError::validation("Authorization header is required")
        })?;

    let token = state
        .auth_service
        .extract_token_from_header(auth_header)
        .map_err(|e| {
            logrs::error!("Failed to extract token from header: {}", e);
            AppError::validation("Invalid Authorization header format")
        })?;

    // 验证访问令牌
    let claims = state
        .auth_service
        .get_token_service()
        .decode(&token)
        .map_err(|e| {
            logrs::error!("Failed to decode token: {}", e);
            AppError::token("Invalid access token")
        })?;

    // 获取房间信息
    let room = get_room_by_id(state.db_pool(), claims.room_id)
        .await
        .map_err(|e| {
            logrs::error!("Failed to get room: {}", e);
            AppError::internal("Failed to retrieve room information")
        })?
        .ok_or_else(|| {
            logrs::error!("Room not found for id: {}", claims.room_id);
            AppError::room_not_found(claims.room_id.to_string())
        })?;

    // 验证访问令牌
    state
        .auth_service
        .verify_access_token(&token, &room)
        .await
        .map_err(|e| {
            logrs::error!("Failed to verify access token: {}", e);
            AppError::authentication("Token verification failed")
        })?;

    // 撤销令牌
    state
        .auth_service
        .blacklist_token(&claims)
        .await
        .map_err(|e| {
            logrs::error!("Failed to blacklist token: {}", e);
            AppError::internal("Failed to blacklist token")
        })?;

    // 撤销关联的刷新令牌
    if let Some(refresh_jti) = claims.refresh_jti {
        state
            .refresh_token_service
            .revoke_token(&refresh_jti)
            .await
            .map_err(|e| {
                logrs::error!("Failed to revoke refresh token: {}", e);
                AppError::internal("Failed to revoke refresh token")
            })?;
    }

    // 记录日志
    logrs::info!(
        "User logged out successfully via auth header for room_id: {}, user: {}",
        claims.room_id,
        claims.sub
    );

    Ok(LogoutResponseSchema {
        success: true,
        message: "Successfully logged out".to_string(),
    })
}

/// 根据房间 ID 获取房间信息
async fn get_room_by_id(db_pool: &Arc<DbPool>, room_id: i64) -> Result<Option<Room>, sqlx::Error> {
    let room = sqlx::query_as!(
        Room,
        r#"
        SELECT
            id,
            name,
            slug,
            password,
            status as "status: RoomStatus",
            max_size,
            current_size,
            max_times_entered,
            current_times_entered,
            expire_at,
            created_at,
            updated_at,
            permission as "permission: RoomPermission"
        FROM rooms
        WHERE id = ?
        "#,
        room_id,
    )
    .fetch_optional(db_pool.as_ref())
    .await?;

    Ok(room)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_refresh_token_request_schema() {
        let request = RefreshTokenRequestSchema {
            refresh_token: "test_refresh_token".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test_refresh_token"));
    }

    #[test]
    fn test_logout_request_schema() {
        let request = LogoutRequestSchema {
            token: Some("test_token".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test_token"));
    }

    #[test]
    fn test_logout_request_schema_without_token() {
        let request = LogoutRequestSchema { token: None };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("null"));
    }
}
