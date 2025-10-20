use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Executor, Sqlite};
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::RoomToken;

#[async_trait]
pub trait IRoomTokenRepository: Send + Sync {
    async fn create(&self, room_token: &RoomToken) -> Result<RoomToken>;
    async fn find_by_jti(&self, jti: &str) -> Result<Option<RoomToken>>;
    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomToken>>;
    async fn revoke(&self, jti: &str) -> Result<bool>;
    async fn delete_by_room(&self, room_id: i64) -> Result<u64>;
}

pub struct SqliteRoomTokenRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomTokenRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional_by_jti<'e, E>(executor: E, jti: &str) -> Result<Option<RoomToken>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let token = sqlx::query_as!(
            RoomToken,
            r#"
            SELECT
                id,
                room_id,
                jti,
                expires_at,
                revoked_at,
                created_at
            FROM room_tokens
            WHERE jti = ?
            "#,
            jti
        )
        .fetch_optional(executor)
        .await?;

        Ok(token)
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<RoomToken>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as!(
            RoomToken,
            r#"
            SELECT
                id,
                room_id,
                jti,
                expires_at,
                revoked_at,
                created_at
            FROM room_tokens
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| anyhow!("room token not found for id {}", id))
    }
}

#[async_trait]
impl IRoomTokenRepository for SqliteRoomTokenRepository {
    async fn create(&self, room_token: &RoomToken) -> Result<RoomToken> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();

        let insert = sqlx::query!(
            r#"
            INSERT INTO room_tokens (
                room_id, jti, expires_at, revoked_at, created_at
            ) VALUES (?, ?, ?, ?, ?)
            "#,
            room_token.room_id,
            room_token.jti,
            room_token.expires_at,
            room_token.revoked_at,
            now
        )
        .execute(&mut *tx)
        .await?;

        let token = Self::fetch_by_id_or_err(&mut *tx, insert.last_insert_rowid()).await?;
        tx.commit().await?;
        Ok(token)
    }

    async fn find_by_jti(&self, jti: &str) -> Result<Option<RoomToken>> {
        Self::fetch_optional_by_jti(&*self.pool, jti).await
    }

    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomToken>> {
        let tokens = sqlx::query_as!(
            RoomToken,
            r#"
            SELECT
                id,
                room_id,
                jti,
                expires_at,
                revoked_at,
                created_at
            FROM room_tokens
            WHERE room_id = ?
            ORDER BY created_at DESC
            "#,
            room_id
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(tokens)
    }

    async fn revoke(&self, jti: &str) -> Result<bool> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query!(
            r#"
            UPDATE room_tokens
            SET revoked_at = ?
            WHERE jti = ? AND revoked_at IS NULL
            "#,
            now,
            jti
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete_by_room(&self, room_id: i64) -> Result<u64> {
        let result = sqlx::query!("DELETE FROM room_tokens WHERE room_id = ?", room_id)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
