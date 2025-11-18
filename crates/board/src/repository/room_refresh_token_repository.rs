use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Any, AnyPool};
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::refresh_token::{RoomRefreshToken, TokenBlacklistEntry};

#[async_trait]
pub trait IRoomRefreshTokenRepository: Send + Sync {
    async fn create(&self, token: &RoomRefreshToken) -> Result<RoomRefreshToken>;
    async fn find_by_token(&self, token: &str) -> Result<Option<RoomRefreshToken>>;
    async fn list_active(&self, room_id: i64) -> Result<Vec<RoomRefreshToken>>;
    async fn delete_by_room(&self, room_id: i64) -> Result<u64>;
    async fn delete_expired(&self) -> Result<u64>;
    async fn blacklist(&self, entry: &TokenBlacklistEntry) -> Result<TokenBlacklistEntry>;
    async fn is_blacklisted(&self, token: &str) -> Result<bool>;
    async fn purge_blacklist(&self) -> Result<u64>;
}

pub struct SqliteRoomRefreshTokenRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomRefreshTokenRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional<'e, E>(executor: E, token: &str) -> Result<Option<RoomRefreshToken>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomRefreshToken>(
            r#"
            SELECT id, room_id, token, created_at, expires_at
            FROM room_refresh_tokens
            WHERE token = ?
            "#,
        )
        .bind(token)
        .fetch_optional(executor)
        .await
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<RoomRefreshToken>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomRefreshToken>(
            r#"
            SELECT id, room_id, token, created_at, expires_at
            FROM room_refresh_tokens
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| anyhow!("refresh token not found"))
    }
}

#[async_trait]
impl IRoomRefreshTokenRepository for SqliteRoomRefreshTokenRepository {
    async fn create(&self, token: &RoomRefreshToken) -> Result<RoomRefreshToken> {
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_refresh_tokens (room_id, token, created_at, expires_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(token.room_id)
        .bind(&token.token)
        .bind(token.created_at)
        .bind(token.expires_at)
        .fetch_one(&mut *tx)
        .await?;

        let created = Self::fetch_by_id_or_err(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<RoomRefreshToken>> {
        Self::fetch_optional(&*self.pool, token).await
    }

    async fn list_active(&self, room_id: i64) -> Result<Vec<RoomRefreshToken>> {
        sqlx::query_as::<_, RoomRefreshToken>(
            r#"
            SELECT id, room_id, token, created_at, expires_at
            FROM room_refresh_tokens
            WHERE room_id = ? AND expires_at > ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(room_id)
        .bind(Utc::now().naive_utc())
        .fetch_all(&*self.pool)
        .await
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

    async fn blacklist(&self, entry: &TokenBlacklistEntry) -> Result<TokenBlacklistEntry> {
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO token_blacklist (token, reason, expires_at, created_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&entry.token)
        .bind(&entry.reason)
        .bind(entry.expires_at)
        .bind(entry.created_at)
        .fetch_one(&mut *tx)
        .await?;

        let created = sqlx::query_as::<_, TokenBlacklistEntry>(
            r#"
            SELECT id, token, reason, expires_at, created_at
            FROM token_blacklist
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(created)
    }

    async fn is_blacklisted(&self, token: &str) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM token_blacklist WHERE token = ? AND expires_at > ?)",
        )
        .bind(token)
        .bind(Utc::now().naive_utc())
        .fetch_one(&*self.pool)
        .await?;
        Ok(exists != 0)
    }

    async fn purge_blacklist(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM token_blacklist WHERE expires_at <= ?")
            .bind(Utc::now().naive_utc())
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
