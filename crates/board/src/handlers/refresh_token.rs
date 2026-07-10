use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use crate::dto::auth::{CleanupResponse, LogoutRequest};
use crate::errors::{AppError, AppResult};
use crate::models::{RefreshTokenRequest, RefreshTokenResponse};
use crate::state::AppState;

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
    let response = app_state
        .refresh_token_service()
        .refresh_access_token(&request.refresh_token)
        .await
        .map_err(|error| {
            logrs::error!("Failed to refresh access token: {error}");
            AppError::authentication("Invalid or expired refresh token")
        })?;

    Ok(Json(response))
}

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
    let claims = app_state
        .token_service()
        .decode(&request.access_token)
        .map_err(|_| AppError::authentication("Invalid access token"))?;

    app_state
        .refresh_token_service()
        .revoke_token(&claims.jti)
        .await
        .map_err(|error| {
            logrs::error!("Failed to revoke token: {error}");
            AppError::internal("Failed to revoke token")
        })?;
    app_state
        .connection_manager
        .disconnect_room(&claims.room_name, "Session revoked")
        .await;

    Ok("Token revoked successfully".to_string())
}

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
    let cleaned_count = app_state
        .refresh_token_service()
        .cleanup_expired()
        .await
        .map_err(|error| {
            logrs::error!("Failed to cleanup expired tokens: {error}");
            AppError::internal("Failed to cleanup expired tokens")
        })?;

    Ok(Json(CleanupResponse {
        cleaned_records: cleaned_count,
        message: "Cleanup completed successfully".to_string(),
    }))
}
