use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Any;
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

pub struct RoomTokenRepository {
    pool: Arc<DbPool>,
}

impl RoomTokenRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional<'e, E>(executor: E, jti: &str) -> Result<Option<RoomToken>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomToken>(
            r#"
            SELECT id, room_id, jti, expires_at, revoked_at, created_at
            FROM room_tokens
            WHERE jti = ?
            "#,
        )
        .bind(jti)
        .fetch_optional(executor)
        .await
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<RoomToken>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomToken>(
            r#"
            SELECT id, room_id, jti, expires_at, revoked_at, created_at
            FROM room_tokens
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| anyhow!("room token not found"))
    }
}

#[async_trait]
impl IRoomTokenRepository for RoomTokenRepository {
    async fn create(&self, room_token: &RoomToken) -> Result<RoomToken> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_tokens (room_id, jti, expires_at, revoked_at, created_at)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(room_token.room_id)
        .bind(&room_token.jti)
        .bind(room_token.expires_at)
        .bind(room_token.revoked_at)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        let created = Self::fetch_by_id_or_err(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn find_by_jti(&self, jti: &str) -> Result<Option<RoomToken>> {
        Self::fetch_optional(&*self.pool, jti).await
    }

    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomToken>> {
        sqlx::query_as::<_, RoomToken>(
            r#"
            SELECT id, room_id, jti, expires_at, revoked_at, created_at
            FROM room_tokens
            WHERE room_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(room_id)
        .fetch_all(&*self.pool)
        .await
    }

    async fn revoke(&self, jti: &str) -> Result<bool> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query(
            r#"
            UPDATE room_tokens
            SET revoked_at = ?
            WHERE jti = ? AND revoked_at IS NULL
            "#,
        )
        .bind(now)
        .bind(jti)
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_by_room(&self, room_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM room_tokens WHERE room_id = ?")
            .bind(room_id)
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
