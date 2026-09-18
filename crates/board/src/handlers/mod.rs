pub mod admin;
pub mod admin_auth;
pub mod chunked_upload;
pub mod config;
pub mod content;
pub mod refresh_token;
pub mod rooms;
mod token;

use axum::http::HeaderMap;

pub(crate) use token::*;

/// 请求客户端标识：反代头优先，回退常量。
/// 仅用于防爆破按键（AttemptGuard）与日志，不构成授权依据。
pub(crate) fn client_ip(headers: &HeaderMap) -> &str {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("client")
}
