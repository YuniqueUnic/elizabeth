use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Any;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::chunk_upload::{ChunkedUploadStatusResponse, RoomChunkUpload};

#[async_trait]
pub trait IRoomChunkUploadRepository: Send + Sync {
    async fn create(&self, upload: &RoomChunkUpload) -> Result<RoomChunkUpload>;
    async fn find_by_upload_id(&self, upload_id: &str) -> Result<Option<RoomChunkUpload>>;
    async fn update_status(&self, upload_id: &str, uploaded_chunks: i64)
    -> Result<RoomChunkUpload>;
    async fn finalize(&self, upload_id: &str) -> Result<RoomChunkUpload>;
    async fn delete_by_room(&self, room_id: i64) -> Result<u64>;
    async fn status(&self, upload_id: &str) -> Result<Option<ChunkedUploadStatusResponse>>;
}

pub struct RoomChunkUploadRepository {
    pool: Arc<DbPool>,
}

impl RoomChunkUploadRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<RoomChunkUpload>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomChunkUpload>(
            r#"
            SELECT id, room_id, upload_id, file_name, total_chunks, uploaded_chunks,
                   size, mime_type, status as "status: _", created_at, updated_at
            FROM room_chunk_uploads
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| anyhow!("chunk upload not found"))
    }
}

#[async_trait]
impl IRoomChunkUploadRepository for RoomChunkUploadRepository {
    async fn create(&self, upload: &RoomChunkUpload) -> Result<RoomChunkUpload> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_chunk_uploads (
                room_id, upload_id, file_name, total_chunks, uploaded_chunks,
                size, mime_type, status, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(upload.room_id)
        .bind(&upload.upload_id)
        .bind(&upload.file_name)
        .bind(upload.total_chunks)
        .bind(upload.uploaded_chunks)
        .bind(upload.size)
        .bind(&upload.mime_type)
        .bind(upload.status)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        let created = Self::fetch_by_id_or_err(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn find_by_upload_id(&self, upload_id: &str) -> Result<Option<RoomChunkUpload>> {
        sqlx::query_as::<_, RoomChunkUpload>(
            r#"
            SELECT id, room_id, upload_id, file_name, total_chunks, uploaded_chunks,
                   size, mime_type, status as "status: _", created_at, updated_at
            FROM room_chunk_uploads
            WHERE upload_id = ?
            "#,
        )
        .bind(upload_id)
        .fetch_optional(&*self.pool)
        .await
    }

    async fn update_status(
        &self,
        upload_id: &str,
        uploaded_chunks: i64,
    ) -> Result<RoomChunkUpload> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            UPDATE room_chunk_uploads
            SET uploaded_chunks = ?, updated_at = ?
            WHERE upload_id = ?
            "#,
        )
        .bind(uploaded_chunks)
        .bind(Utc::now().naive_utc())
        .bind(upload_id)
        .execute(&mut *tx)
        .await?;

        let refreshed = sqlx::query_as::<_, RoomChunkUpload>(
            r#"
            SELECT id, room_id, upload_id, file_name, total_chunks, uploaded_chunks,
                   size, mime_type, status as "status: _", created_at, updated_at
            FROM room_chunk_uploads
            WHERE upload_id = ?
            "#,
        )
        .bind(upload_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(refreshed)
    }

    async fn finalize(&self, upload_id: &str) -> Result<RoomChunkUpload> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            UPDATE room_chunk_uploads
            SET status = 2, updated_at = ?
            WHERE upload_id = ?
            "#,
        )
        .bind(Utc::now().naive_utc())
        .bind(upload_id)
        .execute(&mut *tx)
        .await?;

        let refreshed = sqlx::query_as::<_, RoomChunkUpload>(
            r#"
            SELECT id, room_id, upload_id, file_name, total_chunks, uploaded_chunks,
                   size, mime_type, status as "status: _", created_at, updated_at
            FROM room_chunk_uploads
            WHERE upload_id = ?
            "#,
        )
        .bind(upload_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(refreshed)
    }

    async fn delete_by_room(&self, room_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM room_chunk_uploads WHERE room_id = ?")
            .bind(room_id)
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn status(&self, upload_id: &str) -> Result<Option<ChunkedUploadStatusResponse>> {
        sqlx::query_as::<_, ChunkedUploadStatusResponse>(
            r#"
            SELECT upload_id, total_chunks, uploaded_chunks, status as "status: _"
            FROM room_chunk_uploads
            WHERE upload_id = ?
            "#,
        )
        .bind(upload_id)
        .fetch_optional(&*self.pool)
        .await
    }
}
