use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Any;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::chunk_upload::RoomChunkUpload;

const SELECT_BASE: &str = r#"
    SELECT
        id,
        reservation_id,
        chunk_index,
        chunk_size,
        chunk_hash,
        upload_status as "upload_status: _",
        created_at,
        updated_at
    FROM room_chunk_uploads
"#;

#[async_trait]
pub trait IRoomChunkUploadRepository: Send + Sync {
    async fn create(&self, upload: &RoomChunkUpload) -> Result<RoomChunkUpload>;
    async fn find_by_reservation_and_index(
        &self,
        reservation_id: i64,
        chunk_index: i64,
    ) -> Result<Option<RoomChunkUpload>>;
    async fn find_by_reservation_id(&self, reservation_id: i64) -> Result<Vec<RoomChunkUpload>>;
    async fn count_by_reservation_id(&self, reservation_id: i64) -> Result<i64>;
    async fn delete_by_room(&self, room_id: i64) -> Result<u64>;
}

pub struct RoomChunkUploadRepository {
    pool: Arc<DbPool>,
}

impl RoomChunkUploadRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_by_id<'e, E>(executor: E, id: i64) -> Result<RoomChunkUpload>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomChunkUpload>(&format!("{SELECT_BASE} WHERE id = ?"))
            .bind(id)
            .fetch_optional(executor)
            .await?
            .ok_or_else(|| anyhow!("chunk upload not found"))
    }

    async fn fetch_optional<'e, E>(
        executor: E,
        reservation_id: i64,
        chunk_index: i64,
    ) -> Result<Option<RoomChunkUpload>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomChunkUpload>(&format!(
            "{SELECT_BASE} WHERE reservation_id = ? AND chunk_index = ?"
        ))
        .bind(reservation_id)
        .bind(chunk_index)
        .fetch_optional(executor)
        .await
    }
}

#[async_trait]
impl IRoomChunkUploadRepository for RoomChunkUploadRepository {
    async fn create(&self, upload: &RoomChunkUpload) -> Result<RoomChunkUpload> {
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_chunk_uploads (
                reservation_id,
                chunk_index,
                chunk_size,
                chunk_hash,
                upload_status,
                created_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(upload.reservation_id)
        .bind(upload.chunk_index)
        .bind(upload.chunk_size)
        .bind(&upload.chunk_hash)
        .bind(upload.upload_status.to_string())
        .bind(upload.created_at)
        .bind(upload.updated_at)
        .fetch_one(&mut *tx)
        .await?;

        let created = Self::fetch_by_id(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn find_by_reservation_and_index(
        &self,
        reservation_id: i64,
        chunk_index: i64,
    ) -> Result<Option<RoomChunkUpload>> {
        Ok(Self::fetch_optional(&*self.pool, reservation_id, chunk_index).await?)
    }

    async fn find_by_reservation_id(&self, reservation_id: i64) -> Result<Vec<RoomChunkUpload>> {
        sqlx::query_as::<_, RoomChunkUpload>(&format!(
            "{SELECT_BASE} WHERE reservation_id = ? ORDER BY chunk_index"
        ))
        .bind(reservation_id)
        .fetch_all(&*self.pool)
        .await
    }

    async fn count_by_reservation_id(&self, reservation_id: i64) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM room_chunk_uploads WHERE reservation_id = ?
            "#,
        )
        .bind(reservation_id)
        .fetch_one(&*self.pool)
        .await?;

        Ok(count)
    }

    async fn delete_by_room(&self, room_id: i64) -> Result<u64> {
        // room_id is stored on reservations; leverage ON DELETE CASCADE but keep helper for explicit cleanup
        let result = sqlx::query(
            r#"
            DELETE FROM room_chunk_uploads
            WHERE reservation_id IN (
                SELECT id FROM room_upload_reservations WHERE room_id = ?
            )
            "#,
        )
        .bind(room_id)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
