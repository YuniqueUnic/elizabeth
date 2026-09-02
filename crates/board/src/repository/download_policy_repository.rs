use async_trait::async_trait;
use board_protocol::models::room::{DownloadPolicyMode, FileAccessCode, FileDownloadPolicy};
use chrono::Utc;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::row_utils::{format_naive_datetime, format_optional_naive_datetime};

#[async_trait]
pub trait IDownloadPolicyRepository: Send + Sync {
    async fn get_policy_by_content_id(
        &self,
        content_id: i64,
    ) -> Result<Option<FileDownloadPolicy>, sqlx::Error>;
    async fn upsert_policy(
        &self,
        policy: &FileDownloadPolicy,
    ) -> Result<FileDownloadPolicy, sqlx::Error>;
    async fn create_access_codes(&self, codes: &[FileAccessCode]) -> Result<(), sqlx::Error>;
    async fn redeem_code(&self, content_id: i64, code_hash: &str) -> Result<bool, sqlx::Error>;
    async fn increment_download_count(&self, content_id: i64) -> Result<(), sqlx::Error>;
    async fn get_code_stats(&self, policy_id: i64) -> Result<(i64, i64, i64), sqlx::Error>;
    async fn replace_access_codes(
        &self,
        policy_id: i64,
        codes: &[FileAccessCode],
    ) -> Result<(), sqlx::Error>;
    async fn clear_access_codes(&self, policy_id: i64) -> Result<(), sqlx::Error>;
}

pub struct DownloadPolicyRepository {
    pool: Arc<DbPool>,
}

impl DownloadPolicyRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IDownloadPolicyRepository for DownloadPolicyRepository {
    async fn get_policy_by_content_id(
        &self,
        content_id: i64,
    ) -> Result<Option<FileDownloadPolicy>, sqlx::Error> {
        let sql = r#"
            SELECT
                id,
                content_id,
                mode,
                max_downloads,
                download_count,
                CAST(created_at AS TEXT) as created_at,
                CAST(updated_at AS TEXT) as updated_at
            FROM file_download_policies
            WHERE content_id = $1
        "#;

        sqlx::query_as(sql)
            .bind(content_id)
            .fetch_optional(self.pool.as_ref())
            .await
    }

    async fn upsert_policy(
        &self,
        policy: &FileDownloadPolicy,
    ) -> Result<FileDownloadPolicy, sqlx::Error> {
        let mode_str = match policy.mode {
            DownloadPolicyMode::Off => "off",
            DownloadPolicyMode::Reusable => "reusable",
            DownloadPolicyMode::OneTime => "one_time",
        };

        let now_str = format_naive_datetime(policy.updated_at);
        let created_str = format_naive_datetime(policy.created_at);

        let sql = r#"
            INSERT INTO file_download_policies (content_id, mode, max_downloads, download_count, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (content_id)
            DO UPDATE SET mode = $2, max_downloads = $3, updated_at = $6
            RETURNING
                id,
                content_id,
                mode,
                max_downloads,
                download_count,
                CAST(created_at AS TEXT) as created_at,
                CAST(updated_at AS TEXT) as updated_at
        "#;

        sqlx::query_as(sql)
            .bind(policy.content_id)
            .bind(mode_str)
            .bind(policy.max_downloads)
            .bind(policy.download_count)
            .bind(created_str)
            .bind(now_str)
            .fetch_one(self.pool.as_ref())
            .await
    }

    async fn create_access_codes(&self, codes: &[FileAccessCode]) -> Result<(), sqlx::Error> {
        if codes.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        let sql = r#"
            INSERT INTO file_access_codes (policy_id, code_hash, is_reusable, used_at, created_at)
            VALUES ($1, $2, $3, $4, $5)
        "#;

        for code in codes {
            let created_str = format_naive_datetime(code.created_at);
            let used_str = format_optional_naive_datetime(code.used_at);
            let is_reusable_val = if code.is_reusable { 1i64 } else { 0i64 };

            sqlx::query(sql)
                .bind(code.policy_id)
                .bind(&code.code_hash)
                .bind(is_reusable_val)
                .bind(used_str)
                .bind(created_str)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn redeem_code(&self, content_id: i64, code_hash: &str) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // 1. Get policy
        let sql_policy = r#"
            SELECT
                id,
                content_id,
                mode,
                max_downloads,
                download_count,
                CAST(created_at AS TEXT) as created_at,
                CAST(updated_at AS TEXT) as updated_at
            FROM file_download_policies
            WHERE content_id = $1
        "#;

        let policy: Option<FileDownloadPolicy> = sqlx::query_as(sql_policy)
            .bind(content_id)
            .fetch_optional(&mut *tx)
            .await?;

        let policy = match policy {
            Some(p) => p,
            None => return Ok(false), // No policy, cannot redeem
        };

        // 2. Check max downloads limit (for reusable mode)
        if policy.mode == DownloadPolicyMode::Reusable
            && let Some(max_dl) = policy.max_downloads
            && policy.download_count >= max_dl
        {
            return Ok(false);
        }

        // 3. Find code
        let sql_code = r#"
            SELECT
                id,
                policy_id,
                code_hash,
                CASE WHEN is_reusable THEN 1 ELSE 0 END as is_reusable,
                CAST(used_at AS TEXT) as used_at,
                CAST(created_at AS TEXT) as created_at
            FROM file_access_codes
            WHERE policy_id = $1 AND code_hash = $2
        "#;

        let code: Option<FileAccessCode> = sqlx::query_as(sql_code)
            .bind(policy.id)
            .bind(code_hash)
            .fetch_optional(&mut *tx)
            .await?;

        let code = match code {
            Some(c) => c,
            None => return Ok(false), // Invalid code
        };

        // 4. Check if already used
        if !code.is_reusable && code.used_at.is_some() {
            return Ok(false);
        }

        // 5. Mark as used
        let now_str = format_naive_datetime(Utc::now().naive_utc());
        let sql_update_code = "UPDATE file_access_codes SET used_at = $1 WHERE id = $2";
        sqlx::query(sql_update_code)
            .bind(now_str)
            .bind(code.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    async fn increment_download_count(&self, content_id: i64) -> Result<(), sqlx::Error> {
        let now_str = format_naive_datetime(Utc::now().naive_utc());
        let sql = "UPDATE file_download_policies SET download_count = download_count + 1, updated_at = $1 WHERE content_id = $2";
        sqlx::query(sql)
            .bind(now_str)
            .bind(content_id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }

    async fn get_code_stats(&self, policy_id: i64) -> Result<(i64, i64, i64), sqlx::Error> {
        let total_codes: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM file_access_codes WHERE policy_id = $1")
                .bind(policy_id)
                .fetch_one(self.pool.as_ref())
                .await?;

        let used_codes: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM file_access_codes WHERE policy_id = $1 AND used_at IS NOT NULL",
        )
        .bind(policy_id)
        .fetch_one(self.pool.as_ref())
        .await?;

        let remaining_codes: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM file_access_codes WHERE policy_id = $1 AND used_at IS NULL",
        )
        .bind(policy_id)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok((total_codes.0, remaining_codes.0, used_codes.0))
    }

    async fn replace_access_codes(
        &self,
        policy_id: i64,
        codes: &[FileAccessCode],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM file_access_codes WHERE policy_id = $1")
            .bind(policy_id)
            .execute(&mut *tx)
            .await?;

        let sql = r#"
            INSERT INTO file_access_codes (policy_id, code_hash, is_reusable, used_at, created_at)
            VALUES ($1, $2, $3, $4, $5)
        "#;

        for code in codes {
            let created_str = format_naive_datetime(code.created_at);
            let used_str = format_optional_naive_datetime(code.used_at);
            let is_reusable_val = if code.is_reusable { 1i64 } else { 0i64 };

            sqlx::query(sql)
                .bind(code.policy_id)
                .bind(&code.code_hash)
                .bind(is_reusable_val)
                .bind(used_str)
                .bind(created_str)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn clear_access_codes(&self, policy_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM file_access_codes WHERE policy_id = $1")
            .bind(policy_id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }
}
