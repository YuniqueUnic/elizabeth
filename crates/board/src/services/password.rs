//! 通用 Argon2 密码哈希基础设施：房间密码与管理员账号共用。

use anyhow::{Context, Result};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use sqlx::Row;

use crate::db::DbPool;

const ARGON2_PREFIX: &str = "$argon2";

#[derive(Debug, Clone, Default)]
pub struct PasswordHashService;

impl PasswordHashService {
    pub async fn hash(&self, password: String) -> Result<String> {
        tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .context("room password hashing task failed")?
    }

    pub async fn verify(&self, password: String, encoded_hash: String) -> Result<bool> {
        tokio::task::spawn_blocking(move || verify_password(&password, &encoded_hash))
            .await
            .context("room password verification task failed")?
    }
}

fn hash_password(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|error| anyhow::anyhow!("failed to hash room password: {error}"))
}

fn verify_password(password: &str, encoded_hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(encoded_hash)
        .map_err(|error| anyhow::anyhow!("invalid room password hash: {error}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// 一次性预计算的假哈希：登录用户名不存在时也走一遍 Argon2 校验，
/// 抹平“账号不存在”与“密码错误”的响应时间差，防止用户枚举。
pub fn decoy_hash() -> String {
    static DECOY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    DECOY
        .get_or_init(|| {
            hash_password("elizabeth-decoy-password-for-timing-equalization")
                .expect("decoy hash generation cannot fail")
        })
        .clone()
}

/// One-time, idempotent data upgrade for volumes created before password hashing.
///
/// SQL migrations cannot generate Argon2 salts, so the server performs this data
/// rewrite immediately after schema migrations and before accepting traffic.
pub async fn migrate_legacy_room_passwords(
    pool: &DbPool,
    service: &PasswordHashService,
) -> Result<u64> {
    let rows = sqlx::query(
        "SELECT id, password FROM rooms WHERE password IS NOT NULL AND password NOT LIKE '$argon2%'",
    )
    .fetch_all(pool)
    .await
    .context("failed to load legacy room passwords")?;

    let mut migrated = 0;
    for row in rows {
        let room_id: i64 = row.try_get("id")?;
        let password: String = row.try_get("password")?;
        let encoded_hash = service.hash(password).await?;
        let result = sqlx::query(
            "UPDATE rooms SET password = $1 WHERE id = $2 AND password NOT LIKE '$argon2%'",
        )
        .bind(encoded_hash)
        .bind(room_id)
        .execute(pool)
        .await?;
        migrated += result.rows_affected();
    }

    Ok(migrated)
}

pub fn is_encoded_password(value: &str) -> bool {
    value.starts_with(ARGON2_PREFIX)
}
