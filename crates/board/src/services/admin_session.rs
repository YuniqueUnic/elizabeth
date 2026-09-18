//! 管理员登录会话：无状态 JWT，与房间令牌共用签名密钥但声明结构完全隔离。
//!
//! 会话携带签发时的密码版本号（账号 `updated_at` 时间戳）；改密后旧会话
//! 在校验阶段即被拒绝，无需黑名单。有效期使用部署的 `jwt.ttl_seconds`。

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 管理会话声明；`scope` 固定为 `admin`，与房间令牌互不通用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSessionClaims {
    /// 管理员用户名
    pub sub: String,
    pub scope: String,
    pub jti: String,
    /// 签发时的密码版本号（admin_accounts.updated_at 的 unix 秒）
    pub pwdv: i64,
    pub exp: i64,
    pub iat: i64,
}

const ADMIN_SCOPE: &str = "admin";

#[derive(Clone)]
pub struct AdminSessionService {
    secret: Arc<String>,
    ttl: Duration,
    leeway: i64,
}

impl AdminSessionService {
    pub fn new(secret: Arc<String>, ttl_seconds: i64, leeway_seconds: i64) -> Self {
        Self {
            secret,
            ttl: Duration::seconds(ttl_seconds.max(1)),
            leeway: leeway_seconds.max(0),
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn issue(&self, username: &str, password_version: i64) -> Result<(String, i64)> {
        let now = Utc::now();
        let expires_at = now + self.ttl;
        let claims = AdminSessionClaims {
            sub: username.to_owned(),
            scope: ADMIN_SCOPE.to_owned(),
            jti: uuid::Uuid::new_v4().to_string(),
            pwdv: password_version,
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
        };
        let token = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .context("failed to sign admin session token")?;
        Ok((token, expires_at.timestamp()))
    }

    /// 验签并校验 scope；密码版本与账号一致性由调用方比对账号当前状态。
    pub fn verify(&self, token: &str) -> Result<AdminSessionClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = self.leeway as u64;
        validation.required_spec_claims = ["exp"].into_iter().map(String::from).collect();
        let claims = jsonwebtoken::decode::<AdminSessionClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .context("admin session token is invalid or expired")?
        .claims;
        if claims.scope != ADMIN_SCOPE {
            anyhow::bail!("token is not an admin session");
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> AdminSessionService {
        AdminSessionService::new(
            Arc::new("test-secret-key-for-unit-testing-123".to_owned()),
            600,
            5,
        )
    }

    #[test]
    fn issue_then_verify_roundtrips_claims() {
        let (token, exp) = service().issue("admin", 12345).unwrap();
        let claims = service().verify(&token).unwrap();
        assert_eq!(claims.sub, "admin");
        assert_eq!(claims.scope, "admin");
        assert_eq!(claims.pwdv, 12345);
        assert_eq!(claims.exp, exp);
    }

    #[test]
    fn room_token_is_rejected_as_admin_session() {
        // 房间令牌没有 admin scope 字段，验签通过后必须被 scope 检查拒绝
        let room_service = crate::services::RoomTokenService::new(Arc::new(
            "test-secret-key-for-unit-testing-123".to_owned(),
        ));
        let mut room = crate::models::Room::new("room".to_owned(), None);
        room.id = Some(1);
        let (room_token, _) = room_service.issue(&room, "reader").unwrap();
        assert!(service().verify(&room_token).is_err());
    }

    #[test]
    fn tampered_secret_is_rejected() {
        let (token, _) = service().issue("admin", 1).unwrap();
        let other = AdminSessionService::new(
            Arc::new("another-secret-value-0123456789".to_owned()),
            600,
            5,
        );
        assert!(other.verify(&token).is_err());
    }
}
