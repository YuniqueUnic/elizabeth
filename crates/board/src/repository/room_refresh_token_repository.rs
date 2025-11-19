use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Any;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::refresh_token::{RoomRefreshToken, TokenBlacklistEntry};
use crate::models::room::row_utils::{format_naive_datetime, format_optional_naive_datetime};

const REFRESH_TOKEN_SELECT: &str = r#"
    SELECT id,
           room_id,
           access_token_jti,
           token_hash,
           CAST(expires_at AS TEXT) as expires_at,
           CAST(created_at AS TEXT) as created_at,
           CAST(last_used_at AS TEXT) as last_used_at,
           CAST(is_revoked AS INTEGER) as is_revoked
    FROM room_refresh_tokens
"#;

const TOKEN_BLACKLIST_SELECT: &str = r#"
    SELECT id,
           jti,
           CAST(expires_at AS TEXT) as expires_at,
           CAST(created_at AS TEXT) as created_at
    FROM token_blacklist
"#;

#[async_trait]
pub trait IRoomRefreshTokenRepository: Send + Sync {
    async fn create(&self, token: &RoomRefreshToken) -> Result<RoomRefreshToken>;
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RoomRefreshToken>>;
    async fn list_active(&self, room_id: i64) -> Result<Vec<RoomRefreshToken>>;
    async fn update_last_used(&self, id: i64) -> Result<RoomRefreshToken>;
    async fn revoke(&self, id: i64) -> Result<bool>;
    async fn revoke_by_access_jti(&self, access_jti: &str) -> Result<bool>;
    async fn delete_by_room(&self, room_id: i64) -> Result<u64>;
    async fn delete_expired(&self) -> Result<u64>;
    async fn revoke_expired(&self) -> Result<u64>;
}

#[async_trait]
pub trait ITokenBlacklistRepository: Send + Sync {
    async fn add(&self, entry: &TokenBlacklistEntry) -> Result<TokenBlacklistEntry>;
    async fn is_blacklisted(&self, jti: &str) -> Result<bool>;
    async fn remove_expired(&self) -> Result<u64>;
}

pub struct RoomRefreshTokenRepository {
    pool: Arc<DbPool>,
}

impl RoomRefreshTokenRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_by_id<'e, E>(executor: E, id: i64) -> Result<RoomRefreshToken>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!("{REFRESH_TOKEN_SELECT} WHERE id = ?");
        sqlx::query_as::<_, RoomRefreshToken>(&sql)
            .bind(id)
            .fetch_optional(executor)
            .await?
            .ok_or_else(|| anyhow!("refresh token not found"))
    }

    async fn fetch_optional_by_hash<'e, E>(
        executor: E,
        token_hash: &str,
    ) -> Result<Option<RoomRefreshToken>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!("{REFRESH_TOKEN_SELECT} WHERE token_hash = ?");
        let token = sqlx::query_as::<_, RoomRefreshToken>(&sql)
            .bind(token_hash)
            .fetch_optional(executor)
            .await?;
        Ok(token)
    }
}

#[async_trait]
impl IRoomRefreshTokenRepository for RoomRefreshTokenRepository {
    async fn create(&self, token: &RoomRefreshToken) -> Result<RoomRefreshToken> {
        let mut tx = self.pool.begin().await?;
        let expires_at = format_naive_datetime(token.expires_at);
        let created_at = format_naive_datetime(token.created_at);
        let last_used_at = format_optional_naive_datetime(token.last_used_at);
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_refresh_tokens (
                room_id,
                access_token_jti,
                token_hash,
                expires_at,
                created_at,
                last_used_at,
                is_revoked
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(token.room_id)
        .bind(&token.access_token_jti)
        .bind(&token.token_hash)
        .bind(expires_at)
        .bind(created_at)
        .bind(last_used_at)
        .bind(token.is_revoked)
        .fetch_one(&mut *tx)
        .await?;

        let created = Self::fetch_by_id(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RoomRefreshToken>> {
        Self::fetch_optional_by_hash(&*self.pool, token_hash).await
    }

    async fn list_active(&self, room_id: i64) -> Result<Vec<RoomRefreshToken>> {
        let sql = format!(
            "{REFRESH_TOKEN_SELECT} WHERE room_id = ? \
             AND is_revoked = FALSE \
             AND CAST(expires_at AS TEXT) > ? \
             ORDER BY created_at DESC"
        );
        let tokens = sqlx::query_as::<_, RoomRefreshToken>(&sql)
            .bind(room_id)
            .bind(format_naive_datetime(Utc::now().naive_utc()))
            .fetch_all(&*self.pool)
            .await?;
        Ok(tokens)
    }

    async fn update_last_used(&self, id: i64) -> Result<RoomRefreshToken> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query(
            r#"
            UPDATE room_refresh_tokens
            SET last_used_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        let updated = Self::fetch_by_id(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn revoke(&self, id: i64) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE room_refresh_tokens
            SET is_revoked = TRUE,
                last_used_at = COALESCE(last_used_at, ?)
            WHERE id = ? AND is_revoked = FALSE
            "#,
        )
        .bind(format_naive_datetime(Utc::now().naive_utc()))
        .bind(id)
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn revoke_by_access_jti(&self, access_jti: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE room_refresh_tokens
            SET is_revoked = TRUE,
                last_used_at = COALESCE(last_used_at, ?)
            WHERE access_token_jti = ? AND is_revoked = FALSE
            "#,
        )
        .bind(format_naive_datetime(Utc::now().naive_utc()))
        .bind(access_jti)
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_by_room(&self, room_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM room_refresh_tokens WHERE room_id = ?")
            .bind(room_id)
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn delete_expired(&self) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM room_refresh_tokens WHERE CAST(expires_at AS TEXT) <= ?")
                .bind(format_naive_datetime(Utc::now().naive_utc()))
                .execute(&*self.pool)
                .await?;
        Ok(result.rows_affected())
    }

    async fn revoke_expired(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE room_refresh_tokens
            SET is_revoked = TRUE
            WHERE CAST(expires_at AS TEXT) <= ? AND is_revoked = FALSE
            "#,
        )
        .bind(format_naive_datetime(Utc::now().naive_utc()))
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

pub struct TokenBlacklistRepository {
    pool: Arc<DbPool>,
}

impl TokenBlacklistRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ITokenBlacklistRepository for TokenBlacklistRepository {
    async fn add(&self, entry: &TokenBlacklistEntry) -> Result<TokenBlacklistEntry> {
        let mut tx = self.pool.begin().await?;
        let expires_at = format_naive_datetime(entry.expires_at);
        let created_at = format_naive_datetime(entry.created_at);
        sqlx::query(
            r#"
            INSERT INTO token_blacklist (jti, expires_at, created_at)
            VALUES (?, ?, ?)
            ON CONFLICT(jti) DO UPDATE SET
                expires_at = excluded.expires_at,
                created_at = excluded.created_at
            "#,
        )
        .bind(&entry.jti)
        .bind(expires_at.clone())
        .bind(created_at.clone())
        .execute(&mut *tx)
        .await?;

        let sql = format!("{TOKEN_BLACKLIST_SELECT} WHERE jti = ?");
        let stored = sqlx::query_as::<_, TokenBlacklistEntry>(&sql)
            .bind(&entry.jti)
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(stored)
    }

    async fn is_blacklisted(&self, jti: &str) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM token_blacklist WHERE jti = ? AND CAST(expires_at AS TEXT) > ?)",
        )
        .bind(jti)
        .bind(format_naive_datetime(Utc::now().naive_utc()))
        .fetch_one(&*self.pool)
        .await?;
        Ok(exists != 0)
    }

    async fn remove_expired(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM token_blacklist WHERE CAST(expires_at AS TEXT) <= ?")
            .bind(format_naive_datetime(Utc::now().naive_utc()))
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
