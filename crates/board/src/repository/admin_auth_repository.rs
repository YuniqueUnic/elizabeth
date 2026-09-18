use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::row_utils::{format_naive_datetime, format_optional_naive_datetime};
use crate::models::{AdminAccount, AdminApiKey};

const ADMIN_ACCOUNT_SELECT: &str = r#"
    SELECT id, username, password_hash,
           CAST(created_at AS TEXT) AS created_at,
           CAST(updated_at AS TEXT) AS updated_at
    FROM admin_accounts
"#;

const ADMIN_API_KEY_SELECT: &str = r#"
    SELECT id, name, prefix, key_hash,
           CAST(expires_at AS TEXT) AS expires_at,
           CAST(revoked_at AS TEXT) AS revoked_at,
           CAST(created_at AS TEXT) AS created_at,
           CAST(last_used_at AS TEXT) AS last_used_at
    FROM admin_api_keys
"#;

#[async_trait]
pub trait IAdminAccountRepository: Send + Sync {
    async fn count(&self) -> Result<i64>;
    async fn find_by_username(&self, username: &str) -> Result<Option<AdminAccount>>;
    async fn create(&self, username: &str, password_hash: String) -> Result<AdminAccount>;
    /// 更新密码哈希并刷新 `updated_at`（即密码版本号，旧会话随之失效）。
    async fn update_password(&self, id: i64, password_hash: String) -> Result<AdminAccount>;
}

pub struct AdminAccountRepository {
    pool: Arc<DbPool>,
}

impl AdminAccountRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IAdminAccountRepository for AdminAccountRepository {
    async fn count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM admin_accounts")
            .fetch_one(&*self.pool)
            .await?;
        Ok(count)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<AdminAccount>> {
        let sql = format!("{ADMIN_ACCOUNT_SELECT} WHERE username = $1");
        Ok(sqlx::query_as::<_, AdminAccount>(sqlx::AssertSqlSafe(sql))
            .bind(username)
            .fetch_optional(&*self.pool)
            .await?)
    }

    async fn create(&self, username: &str, password_hash: String) -> Result<AdminAccount> {
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO admin_accounts (username, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind(username)
        .bind(&password_hash)
        .fetch_one(&mut *tx)
        .await?;
        let sql = format!("{ADMIN_ACCOUNT_SELECT} WHERE id = $1");
        let created = sqlx::query_as::<_, AdminAccount>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn update_password(&self, id: i64, password_hash: String) -> Result<AdminAccount> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        let result = sqlx::query(
            "UPDATE admin_accounts SET password_hash = $2, updated_at = $3 WHERE id = $1",
        )
        .bind(id)
        .bind(&password_hash)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(anyhow!("admin account not found"));
        }
        let sql = format!("{ADMIN_ACCOUNT_SELECT} WHERE id = $1");
        let updated = sqlx::query_as::<_, AdminAccount>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(updated)
    }
}

#[async_trait]
pub trait IAdminApiKeyRepository: Send + Sync {
    async fn create(
        &self,
        name: &str,
        prefix: &str,
        key_hash: String,
        expires_at: Option<NaiveDateTime>,
    ) -> Result<AdminApiKey>;
    /// 未吊销的 key（含已过期条目，便于管理员发现并清理）。
    async fn list_active(&self) -> Result<Vec<AdminApiKey>>;
    async fn find_active_by_prefix(&self, prefix: &str) -> Result<Option<AdminApiKey>>;
    async fn touch_last_used(&self, id: i64) -> Result<()>;
    async fn revoke(&self, id: i64) -> Result<bool>;
}

pub struct AdminApiKeyRepository {
    pool: Arc<DbPool>,
}

impl AdminApiKeyRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IAdminApiKeyRepository for AdminApiKeyRepository {
    async fn create(
        &self,
        name: &str,
        prefix: &str,
        key_hash: String,
        expires_at: Option<NaiveDateTime>,
    ) -> Result<AdminApiKey> {
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO admin_api_keys (name, prefix, key_hash, expires_at) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(name)
        .bind(prefix)
        .bind(&key_hash)
        .bind(format_optional_naive_datetime(expires_at))
        .fetch_one(&mut *tx)
        .await?;
        let sql = format!("{ADMIN_API_KEY_SELECT} WHERE id = $1");
        let created = sqlx::query_as::<_, AdminApiKey>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn list_active(&self) -> Result<Vec<AdminApiKey>> {
        let sql =
            format!("{ADMIN_API_KEY_SELECT} WHERE revoked_at IS NULL ORDER BY created_at DESC");
        Ok(sqlx::query_as::<_, AdminApiKey>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&*self.pool)
            .await?)
    }

    async fn find_active_by_prefix(&self, prefix: &str) -> Result<Option<AdminApiKey>> {
        let sql = format!("{ADMIN_API_KEY_SELECT} WHERE prefix = $1 AND revoked_at IS NULL");
        Ok(sqlx::query_as::<_, AdminApiKey>(sqlx::AssertSqlSafe(sql))
            .bind(prefix)
            .fetch_optional(&*self.pool)
            .await?)
    }

    async fn touch_last_used(&self, id: i64) -> Result<()> {
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query("UPDATE admin_api_keys SET last_used_at = $2 WHERE id = $1")
            .bind(id)
            .bind(now)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn revoke(&self, id: i64) -> Result<bool> {
        let now = format_naive_datetime(Utc::now().naive_utc());
        let result = sqlx::query(
            "UPDATE admin_api_keys SET revoked_at = $2 WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(id)
        .bind(now)
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
