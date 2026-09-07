use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sqlx::Any;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::RoomIdentityCode;
use crate::models::room::row_utils::{format_naive_datetime, format_optional_naive_datetime};

const IDENTITY_CODE_SELECT: &str = r#"
    SELECT id, room_id, code_hash, role_key,
           CAST(expires_at AS TEXT) AS expires_at,
           CAST(revoked_at AS TEXT) AS revoked_at,
           created_by_jti,
           CAST(created_at AS TEXT) AS created_at,
           CAST(updated_at AS TEXT) AS updated_at
    FROM room_identity_codes
"#;

#[async_trait]
pub trait IRoomIdentityCodeRepository: Send + Sync {
    async fn create(&self, code: &RoomIdentityCode) -> Result<RoomIdentityCode>;
    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomIdentityCode>>;
    async fn find_by_id(&self, room_id: i64, id: i64) -> Result<Option<RoomIdentityCode>>;
    async fn reset(
        &self,
        room_id: i64,
        id: i64,
        code_hash: String,
        expires_at: NaiveDateTime,
    ) -> Result<RoomIdentityCode>;
    async fn update_expiry(
        &self,
        room_id: i64,
        id: i64,
        expires_at: NaiveDateTime,
    ) -> Result<RoomIdentityCode>;
    async fn revoke(&self, room_id: i64, id: i64) -> Result<bool>;
}

pub struct RoomIdentityCodeRepository {
    pool: Arc<DbPool>,
}

impl RoomIdentityCodeRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional<'e, E>(
        executor: E,
        room_id: i64,
        id: i64,
    ) -> Result<Option<RoomIdentityCode>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!("{IDENTITY_CODE_SELECT} WHERE room_id = $1 AND id = $2");
        Ok(sqlx::query_as::<_, RoomIdentityCode>(&sql)
            .bind(room_id)
            .bind(id)
            .fetch_optional(executor)
            .await?)
    }

    async fn fetch_or_error<'e, E>(executor: E, room_id: i64, id: i64) -> Result<RoomIdentityCode>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        Self::fetch_optional(executor, room_id, id)
            .await?
            .ok_or_else(|| anyhow!("room identity code not found"))
    }
}

#[async_trait]
impl IRoomIdentityCodeRepository for RoomIdentityCodeRepository {
    async fn create(&self, code: &RoomIdentityCode) -> Result<RoomIdentityCode> {
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO room_identity_codes (room_id, code_hash, role_key, expires_at, revoked_at, created_by_jti, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
        )
        .bind(code.room_id).bind(&code.code_hash).bind(&code.role_key)
        .bind(format_naive_datetime(code.expires_at)).bind(format_optional_naive_datetime(code.revoked_at))
        .bind(&code.created_by_jti).bind(format_naive_datetime(code.created_at)).bind(format_naive_datetime(code.updated_at))
        .fetch_one(&mut *tx).await?;
        let created = Self::fetch_or_error(&mut *tx, code.room_id, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomIdentityCode>> {
        let sql = format!("{IDENTITY_CODE_SELECT} WHERE room_id = $1 ORDER BY created_at DESC");
        Ok(sqlx::query_as::<_, RoomIdentityCode>(&sql)
            .bind(room_id)
            .fetch_all(&*self.pool)
            .await?)
    }

    async fn find_by_id(&self, room_id: i64, id: i64) -> Result<Option<RoomIdentityCode>> {
        Self::fetch_optional(&*self.pool, room_id, id).await
    }

    async fn reset(
        &self,
        room_id: i64,
        id: i64,
        code_hash: String,
        expires_at: NaiveDateTime,
    ) -> Result<RoomIdentityCode> {
        let now = format_naive_datetime(Utc::now().naive_utc());
        let result = sqlx::query("UPDATE room_identity_codes SET code_hash = $3, expires_at = $4, revoked_at = NULL, updated_at = $5 WHERE room_id = $1 AND id = $2")
            .bind(room_id).bind(id).bind(code_hash).bind(format_naive_datetime(expires_at)).bind(now).execute(&*self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(anyhow!("room identity code not found"));
        }
        Self::fetch_or_error(&*self.pool, room_id, id).await
    }

    async fn update_expiry(
        &self,
        room_id: i64,
        id: i64,
        expires_at: NaiveDateTime,
    ) -> Result<RoomIdentityCode> {
        let now = format_naive_datetime(Utc::now().naive_utc());
        let result = sqlx::query("UPDATE room_identity_codes SET expires_at = $3, updated_at = $4 WHERE room_id = $1 AND id = $2")
            .bind(room_id).bind(id).bind(format_naive_datetime(expires_at)).bind(now).execute(&*self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(anyhow!("room identity code not found"));
        }
        Self::fetch_or_error(&*self.pool, room_id, id).await
    }

    async fn revoke(&self, room_id: i64, id: i64) -> Result<bool> {
        let now = format_naive_datetime(Utc::now().naive_utc());
        let result = sqlx::query("UPDATE room_identity_codes SET revoked_at = $3, updated_at = $3 WHERE room_id = $1 AND id = $2 AND revoked_at IS NULL")
            .bind(room_id).bind(id).bind(now).execute(&*self.pool).await?;
        Ok(result.rows_affected() > 0)
    }
}
