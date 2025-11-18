use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Any;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::refresh_token::{RoomRefreshToken, TokenBlacklistEntry};

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
        sqlx::query_as::<_, RoomRefreshToken>(
            r#"
            SELECT id,
                   room_id,
                   access_token_jti,
                   token_hash,
                   expires_at,
                   created_at,
                   last_used_at,
                   is_revoked
            FROM room_refresh_tokens
            WHERE id = ?
            "#,
        )
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
        sqlx::query_as::<_, RoomRefreshToken>(
            r#"
            SELECT id,
                   room_id,
                   access_token_jti,
                   token_hash,
                   expires_at,
                   created_at,
                   last_used_at,
                   is_revoked
            FROM room_refresh_tokens
            WHERE token_hash = ?
            "#,
        )
        .bind(token_hash)
        .fetch_optional(executor)
        .await
    }
}

#[async_trait]
impl IRoomRefreshTokenRepository for RoomRefreshTokenRepository {
    async fn create(&self, token: &RoomRefreshToken) -> Result<RoomRefreshToken> {
        let mut tx = self.pool.begin().await?;
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
        .bind(token.expires_at)
        .bind(token.created_at)
        .bind(token.last_used_at)
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
        sqlx::query_as::<_, RoomRefreshToken>(
            r#"
            SELECT id,
                   room_id,
                   access_token_jti,
                   token_hash,
                   expires_at,
                   created_at,
                   last_used_at,
                   is_revoked
            FROM room_refresh_tokens
            WHERE room_id = ?
              AND is_revoked = FALSE
              AND expires_at > ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(room_id)
        .bind(Utc::now().naive_utc())
        .fetch_all(&*self.pool)
        .await
    }

    async fn update_last_used(&self, id: i64) -> Result<RoomRefreshToken> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            UPDATE room_refresh_tokens
            SET last_used_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Utc::now().naive_utc())
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
        .bind(Utc::now().naive_utc())
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
        .bind(Utc::now().naive_utc())
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
        let result = sqlx::query("DELETE FROM room_refresh_tokens WHERE expires_at <= ?")
            .bind(Utc::now().naive_utc())
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn revoke_expired(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE room_refresh_tokens
            SET is_revoked = TRUE
            WHERE expires_at <= ? AND is_revoked = FALSE
            "#,
        )
        .bind(Utc::now().naive_utc())
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
        .bind(entry.expires_at)
        .bind(entry.created_at)
        .execute(&mut *tx)
        .await?;

        let stored = sqlx::query_as::<_, TokenBlacklistEntry>(
            r#"
            SELECT id, jti, expires_at, created_at
            FROM token_blacklist
            WHERE jti = ?
            "#,
        )
        .bind(&entry.jti)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(stored)
    }

    async fn is_blacklisted(&self, jti: &str) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM token_blacklist WHERE jti = ? AND expires_at > ?)",
        )
        .bind(jti)
        .bind(Utc::now().naive_utc())
        .fetch_one(&*self.pool)
        .await?;
        Ok(exists != 0)
    }

    async fn remove_expired(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM token_blacklist WHERE expires_at <= ?")
            .bind(Utc::now().naive_utc())
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
