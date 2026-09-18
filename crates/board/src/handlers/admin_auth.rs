//! 管理员账号认证与管理 API key：登录会话签发、改密、key 生命周期。
//!
//! 与 `handlers::admin` 的关系：这里提供获取凭证的端点（登录不需要已有凭证），
//! 其余管理端点统一由 [`crate::handlers::admin::ensure_admin`] 鉴权。

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::HeaderMap;
use chrono::DateTime;

use crate::dto::{
    AdminApiKeyCreateRequest, AdminApiKeyCreateResponse, AdminApiKeyView, AdminLoginRequest,
    AdminLoginResponse, AdminPasswordChangeRequest, AdminSessionView,
};
use crate::errors::AppError;
use crate::handlers::admin::{AdminPrincipal, ensure_admin};
use crate::repository::{
    AdminAccountRepository, AdminApiKeyRepository, IAdminAccountRepository, IAdminApiKeyRepository,
};
use crate::services::{GuardScope, admin_auth, validate_password};
use crate::state::AppState;

type HandlerResult<T> = Result<Json<T>, AppError>;

/// 登录防爆破键：IP + 用户名。IP 维度已被 AttemptGuard 的锁定覆盖，
/// 带上用户名可避免同 IP 多账号场景互相干扰。
fn login_client_key(headers: &HeaderMap, username: &str) -> String {
    format!("{}:{username}", crate::client_ip(headers))
}

/// 管理员登录：用户名 + 密码换会话令牌；失败计入防爆破锁定。
#[utoipa::path(
    post,
    path = "/api/v1/admin/auth/login",
    request_body = AdminLoginRequest,
    responses(
        (status = 200, description = "登录成功", body = AdminLoginResponse),
        (status = 401, description = "用户名或密码错误"),
        (status = 429, description = "失败次数过多，已锁定")
    ),
    tag = "admin"
)]
pub async fn admin_login(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<AdminLoginRequest>,
) -> HandlerResult<AdminLoginResponse> {
    let username = payload.username.trim();
    if username.is_empty() {
        return Err(AppError::validation("username must not be empty"));
    }
    let client = login_client_key(&headers, username);
    app_state
        .attempt_guard()
        .check(GuardScope::AdminLogin, &client)?;

    let repo = AdminAccountRepository::new(app_state.db_pool.clone());
    let account = repo
        .find_by_username(username)
        .await
        .map_err(|e| AppError::internal(format!("Failed to load admin account: {e}")))?;
    let verified = match &account {
        Some(account) => app_state
            .password_hash_service()
            .verify(payload.password, account.password_hash.clone())
            .await
            .map_err(|e| AppError::internal(format!("Failed to verify password: {e}")))?,
        // 无此账号时也执行一次哈希校验，抹平用户枚举的时间差
        None => {
            let _ = app_state
                .password_hash_service()
                .verify(payload.password, crate::services::password::decoy_hash())
                .await;
            false
        }
    };

    if !verified {
        app_state
            .attempt_guard()
            .record_failure(GuardScope::AdminLogin, &client);
        return Err(AppError::authentication("Invalid username or password"));
    }
    app_state
        .attempt_guard()
        .record_success(GuardScope::AdminLogin, &client);

    let account = account.expect("verified account must exist");
    let (token, exp) = app_state
        .services
        .admin_session
        .issue(&account.username, account.password_version())
        .map_err(|e| AppError::internal(format!("Failed to issue admin session: {e}")))?;
    let expires_at = DateTime::from_timestamp(exp, 0)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| AppError::internal("Invalid session expiry"))?;
    Ok(Json(AdminLoginResponse {
        username: account.username,
        token,
        expires_at,
    }))
}

/// 当前会话信息：用于前端校验保存的会话是否仍然有效。
#[utoipa::path(
    get,
    path = "/api/v1/admin/auth/me",
    responses(
        (status = 200, description = "会话有效", body = AdminSessionView),
        (status = 401, description = "未携带或凭证无效")
    ),
    tag = "admin"
)]
pub async fn admin_me(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<AdminSessionView> {
    let principal = ensure_admin(&app_state, &headers).await?;
    Ok(Json(AdminSessionView {
        username: principal.username().to_owned(),
    }))
}

/// 修改当前管理员账号密码；改密后该账号所有既有会话立即失效（API key 不受影响）。
#[utoipa::path(
    put,
    path = "/api/v1/admin/auth/password",
    request_body = AdminPasswordChangeRequest,
    responses(
        (status = 200, description = "修改成功，返回当前会话", body = AdminSessionView),
        (status = 400, description = "新密码强度不足"),
        (status = 401, description = "当前密码错误或会话无效"),
        (status = 403, description = "API key 无权改密")
    ),
    tag = "admin"
)]
pub async fn admin_change_password(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<AdminPasswordChangeRequest>,
) -> HandlerResult<AdminSessionView> {
    let AdminPrincipal::Account(account) = ensure_admin(&app_state, &headers).await? else {
        return Err(AppError::permission_denied(
            "API keys cannot change the account password; use the admin account session",
        ));
    };

    let valid_current = app_state
        .password_hash_service()
        .verify(payload.current_password, account.password_hash.clone())
        .await
        .map_err(|e| AppError::internal(format!("Failed to verify password: {e}")))?;
    if !valid_current {
        return Err(AppError::authentication("Current password is incorrect"));
    }
    validate_password(&payload.new_password).map_err(|e| AppError::validation(e.to_string()))?;

    let new_hash = app_state
        .password_hash_service()
        .hash(payload.new_password)
        .await
        .map_err(|e| AppError::internal(format!("Failed to protect password: {e}")))?;
    let updated = AdminAccountRepository::new(app_state.db_pool.clone())
        .update_password(account.id, new_hash)
        .await
        .map_err(|e| AppError::internal(format!("Failed to update password: {e}")))?;
    log::info!(
        "Admin '{}' changed password; existing sessions invalidated",
        updated.username
    );

    Ok(Json(AdminSessionView {
        username: updated.username,
    }))
}

/// 列出未吊销的管理 API key（不回显明文）。
#[utoipa::path(
    get,
    path = "/api/v1/admin/api-keys",
    responses(
        (status = 200, description = "查询成功", body = [AdminApiKeyView]),
        (status = 401, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_list_api_keys(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<AdminApiKeyView>> {
    ensure_admin(&app_state, &headers).await?;
    let keys = AdminApiKeyRepository::new(app_state.db_pool.clone())
        .list_active()
        .await
        .map_err(|e| AppError::internal(format!("Failed to list admin API keys: {e}")))?;
    Ok(Json(keys.into_iter().map(api_key_view).collect()))
}

/// 创建管理 API key：明文仅在本次响应中出现一次。
#[utoipa::path(
    post,
    path = "/api/v1/admin/api-keys",
    request_body = AdminApiKeyCreateRequest,
    responses(
        (status = 200, description = "创建成功", body = AdminApiKeyCreateResponse),
        (status = 400, description = "参数不合法"),
        (status = 401, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_create_api_key(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<AdminApiKeyCreateRequest>,
) -> HandlerResult<AdminApiKeyCreateResponse> {
    ensure_admin(&app_state, &headers).await?;
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::validation("name must not be empty"));
    }
    if name.chars().count() > 100 {
        return Err(AppError::validation("name must be at most 100 characters"));
    }
    if payload.expires_in_secs.is_some_and(|secs| secs <= 0) {
        return Err(AppError::validation("expires_in_secs must be positive"));
    }
    let (key, secret) = admin_auth::mint_api_key(&app_state.db_pool, name, payload.expires_in_secs)
        .await
        .map_err(|e| AppError::validation(e.to_string()))?;
    log::info!("Admin created API key '{}' (id {})", key.name, key.id);
    Ok(Json(AdminApiKeyCreateResponse {
        key: api_key_view(key),
        secret,
    }))
}

/// 吊销管理 API key。
#[utoipa::path(
    delete,
    path = "/api/v1/admin/api-keys/{id}",
    params(("id" = i64, Path, description = "API key id")),
    responses(
        (status = 200, description = "已吊销"),
        (status = 404, description = "key 不存在或已吊销"),
        (status = 401, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_revoke_api_key(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> HandlerResult<()> {
    ensure_admin(&app_state, &headers).await?;
    let revoked = AdminApiKeyRepository::new(app_state.db_pool.clone())
        .revoke(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to revoke admin API key: {e}")))?;
    if !revoked {
        return Err(AppError::not_found("admin API key"));
    }
    Ok(Json(()))
}

fn api_key_view(key: crate::models::AdminApiKey) -> AdminApiKeyView {
    AdminApiKeyView {
        id: key.id,
        name: key.name,
        prefix: key.prefix,
        created_at: key.created_at,
        last_used_at: key.last_used_at,
        expires_at: key.expires_at,
        revoked_at: key.revoked_at,
    }
}
