//! 平台管理员认证：账号 bootstrap、API key 生成与校验。
//!
//! 密码会话（JWT）见 [`crate::services::admin_session`]；防爆破在 handler 层
//! 经 `AttemptGuard::AdminLogin` 统一处理。

use anyhow::{Context, Result, anyhow};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::DbPool;
use crate::models::AdminApiKey;
use crate::repository::{
    AdminAccountRepository, AdminApiKeyRepository, IAdminAccountRepository, IAdminApiKeyRepository,
};
use crate::services::PasswordHashService;

/// 管理员账号 bootstrap 的环境变量：首次启动时创建账号，之后改密一律走 API（持久化）。
pub const ADMIN_USERNAME_ENV: &str = "ELIZABETH_ADMIN_USERNAME";
pub const ADMIN_PASSWORD_ENV: &str = "ELIZABETH_ADMIN_PASSWORD";

/// API key 明文前缀；库中 prefix 列保存的是本前缀之外的随机段头部，用于按前缀定位。
pub const ADMIN_API_KEY_PREFIX: &str = "elizabeth_ak_";
/// API key 随机段中用于库内定位的字符数（其余部分只以哈希形式存在）。
const API_KEY_PREFIX_CHARS: usize = 8;
/// 新管理员密码与 API key 名称的最小长度约束（与旧管理凭证强度要求对齐）。
pub const MIN_ADMIN_PASSWORD_LEN: usize = 12;

/// 启动期 bootstrap：账号表为空且配置了 `ELIZABETH_ADMIN_PASSWORD` 时创建首个管理员账号。
/// 已有账号时环境变量不生效（密码修改经 API 持久化；忘记密码走 CLI 重置）。
pub async fn bootstrap_admin_account(pool: &DbPool) -> Result<bool> {
    let repo = AdminAccountRepository::new(std::sync::Arc::new(pool.clone()));
    if repo.count().await? > 0 {
        return Ok(false);
    }
    let password = std::env::var(ADMIN_PASSWORD_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let Some(password) = password else {
        log::warn!(
            "Admin API disabled: no admin account and {ADMIN_PASSWORD_ENV} is not set; \
             set it once to bootstrap the admin account"
        );
        return Ok(false);
    };
    let username = std::env::var(ADMIN_USERNAME_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "admin".to_owned());
    create_account(pool, &username, &password).await?;
    log::info!("Bootstrapped admin account '{username}' from {ADMIN_PASSWORD_ENV}");
    Ok(true)
}

/// 创建管理员账号（bootstrap 与 CLI 重置共用）。
pub async fn create_account(pool: &DbPool, username: &str, password: &str) -> Result<()> {
    validate_password(password)?;
    let hash = PasswordHashService.hash(password.to_owned()).await?;
    AdminAccountRepository::new(std::sync::Arc::new(pool.clone()))
        .create(username, hash)
        .await
        .map(|_| ())
        .with_context(|| format!("failed to create admin account '{username}'"))
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < MIN_ADMIN_PASSWORD_LEN
        || password.chars().any(char::is_whitespace)
    {
        return Err(anyhow!(
            "password must be at least {MIN_ADMIN_PASSWORD_LEN} characters without whitespace"
        ));
    }
    Ok(())
}

/// 生成一条管理 API key：返回（落库视图，一次性明文）。
pub async fn mint_api_key(
    pool: &DbPool,
    name: &str,
    expires_in_secs: Option<i64>,
) -> Result<(AdminApiKey, String)> {
    let secret = format!(
        "{ADMIN_API_KEY_PREFIX}{}",
        Uuid::new_v4().simple() // 32 个十六进制字符，128 位熵
    );
    // 库内定位前缀：跳过固定前缀，取随机段头部，保证同前缀碰撞概率可忽略且不泄露完整 key
    let prefix: String = secret
        [ADMIN_API_KEY_PREFIX.len()..ADMIN_API_KEY_PREFIX.len() + API_KEY_PREFIX_CHARS]
        .to_owned();
    let key_hash = hash_api_key(&secret);
    let expires_at = match expires_in_secs {
        None => None,
        Some(secs) if secs > 0 => Some(Utc::now().naive_utc() + Duration::seconds(secs)),
        Some(_) => return Err(anyhow!("expires_in_secs must be positive")),
    };
    let created = AdminApiKeyRepository::new(std::sync::Arc::new(pool.clone()))
        .create(name, &prefix, key_hash, expires_at)
        .await?;
    Ok((created, secret))
}

/// 校验 API key 明文：按前缀定位、比对 SHA-256、检查吊销与过期。
pub async fn verify_api_key(pool: &DbPool, secret: &str) -> Result<Option<AdminApiKey>> {
    let shortest = ADMIN_API_KEY_PREFIX.len() + API_KEY_PREFIX_CHARS;
    if !secret.starts_with(ADMIN_API_KEY_PREFIX) || secret.len() < shortest {
        return Ok(None);
    }
    let start = ADMIN_API_KEY_PREFIX.len();
    let prefix = &secret[start..start + API_KEY_PREFIX_CHARS];
    let repo = AdminApiKeyRepository::new(std::sync::Arc::new(pool.clone()));
    let Some(key) = repo.find_active_by_prefix(prefix).await? else {
        return Ok(None);
    };
    if !key.is_active() || key.key_hash != hash_api_key(secret) {
        return Ok(None);
    }
    repo.touch_last_used(key.id).await?;
    Ok(Some(key))
}

fn hash_api_key(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_hash_is_sha256_hex() {
        let hash = hash_api_key("elizabeth_ak_abc");
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, hex::encode(Sha256::digest(b"elizabeth_ak_abc")));
    }

    #[tokio::test]
    async fn mint_rejects_non_positive_expiry() {
        let pool = test_pool().await;
        let err = mint_api_key(&pool, "ci", Some(0)).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn minted_key_verifies_and_invalid_secret_does_not() {
        let pool = test_pool().await;
        let (key, secret) = mint_api_key(&pool, "ci", None).await.unwrap();
        assert!(secret.starts_with(ADMIN_API_KEY_PREFIX));

        let found = verify_api_key(&pool, &secret).await.unwrap().unwrap();
        assert_eq!(found.id, key.id);
        // 校验成功即记录 last_used_at
        let listed = AdminApiKeyRepository::new(std::sync::Arc::new(pool.clone()))
            .list_active()
            .await
            .unwrap();
        assert!(listed.iter().all(|k| k.last_used_at.is_some()));

        assert!(
            verify_api_key(&pool, "elizabeth_ak_deadbeef")
                .await
                .unwrap()
                .is_none()
        );
        assert!(verify_api_key(&pool, "not-a-key").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn revoked_key_no_longer_verifies() {
        let pool = test_pool().await;
        let (key, secret) = mint_api_key(&pool, "ci", None).await.unwrap();
        AdminApiKeyRepository::new(std::sync::Arc::new(pool.clone()))
            .revoke(key.id)
            .await
            .unwrap();
        assert!(verify_api_key(&pool, &secret).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn password_validation_rejects_short_and_blank() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("has whitespace inside").is_err());
        assert!(validate_password("long-enough-password-123").is_ok());
    }

    async fn test_pool() -> crate::db::DbPool {
        // 内存库必须单连接，否则各连接看到的是各自独立的数据库
        let pool = crate::db::DbPoolSettings::new("sqlite::memory:")
            .with_max_connections(1)
            .with_min_connections(1)
            .create_pool()
            .await
            .unwrap();
        crate::db::run_migrations(&pool, "sqlite::memory:")
            .await
            .unwrap();
        pool
    }
}
