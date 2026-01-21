use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Any;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::RoomToken;
use crate::models::room::row_utils::{format_naive_datetime, format_optional_naive_datetime};

const TOKEN_SELECT: &str = r#"
    SELECT id, room_id, jti,
           CAST(expires_at AS TEXT) as expires_at,
           CAST(revoked_at AS TEXT) as revoked_at,
           CAST(created_at AS TEXT) as created_at
    FROM room_tokens
"#;

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
        let sql = format!("{TOKEN_SELECT} WHERE jti = $1");
        let token = sqlx::query_as::<_, RoomToken>(&sql)
            .bind(jti)
            .fetch_optional(executor)
            .await?;
        Ok(token)
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<RoomToken>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!("{TOKEN_SELECT} WHERE id = $1");
        sqlx::query_as::<_, RoomToken>(&sql)
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
        let now_str = format_naive_datetime(now);
        let expires_at = format_naive_datetime(room_token.expires_at);
        let revoked_at = format_optional_naive_datetime(room_token.revoked_at);
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_tokens (room_id, jti, expires_at, revoked_at, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(room_token.room_id)
        .bind(&room_token.jti)
        .bind(expires_at)
        .bind(revoked_at)
        .bind(now_str)
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
        let sql = format!("{TOKEN_SELECT} WHERE room_id = $1 ORDER BY created_at DESC");
        let tokens = sqlx::query_as::<_, RoomToken>(&sql)
            .bind(room_id)
            .fetch_all(&*self.pool)
            .await?;
        Ok(tokens)
    }

    async fn revoke(&self, jti: &str) -> Result<bool> {
        let now = Utc::now().naive_utc();
        let now_str = format_naive_datetime(now);
        let result = sqlx::query(
            r#"
            UPDATE room_tokens
            SET revoked_at = $1
            WHERE jti = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(now_str)
        .bind(jti)
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_by_room(&self, room_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM room_tokens WHERE room_id = $1")
            .bind(room_id)
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
