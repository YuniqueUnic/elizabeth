use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;

use crate::db::DbPool;
use crate::models::room::chunk_upload::{ChunkStatus, RoomChunkUpload};

#[async_trait]
pub trait IRoomChunkUploadRepository: Send + Sync {
    async fn create(&self, chunk: RoomChunkUpload) -> Result<RoomChunkUpload>;
    async fn find_by_reservation_and_index(
        &self,
        reservation_id: i64,
        chunk_index: i64,
    ) -> Result<Option<RoomChunkUpload>>;
    async fn find_by_reservation_id(&self, reservation_id: i64) -> Result<Vec<RoomChunkUpload>>;
    async fn count_by_reservation_id(&self, reservation_id: i64) -> Result<i64>;
}

pub struct SqliteRoomChunkUploadRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomChunkUploadRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IRoomChunkUploadRepository for SqliteRoomChunkUploadRepository {
    async fn create(&self, chunk: RoomChunkUpload) -> Result<RoomChunkUpload> {
        let insert = sqlx::query!(
            r#"
            INSERT INTO room_chunk_uploads
                (reservation_id, chunk_index, chunk_size, chunk_hash, upload_status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            chunk.reservation_id,
            chunk.chunk_index,
            chunk.chunk_size,
            chunk.chunk_hash,
            chunk.upload_status,
            chunk.created_at,
            chunk.updated_at
        )
        .execute(&*self.pool)
        .await?;

        let inserted_id = insert.last_insert_rowid();

        let created_chunk = sqlx::query_as!(
            RoomChunkUpload,
            r#"
            SELECT
                id as "id: Option<i64>",
                reservation_id,
                chunk_index,
                chunk_size,
                chunk_hash,
                upload_status as "upload_status: ChunkStatus",
                created_at,
                updated_at
            FROM room_chunk_uploads
            WHERE id = ?
            "#,
            inserted_id
        )
        .fetch_one(&*self.pool)
        .await?;

        // 修复类型转换问题
        let created_chunk = RoomChunkUpload {
            id: created_chunk.id,
            reservation_id: created_chunk.reservation_id,
            chunk_index: created_chunk.chunk_index,
            chunk_size: created_chunk.chunk_size,
            chunk_hash: created_chunk.chunk_hash,
            upload_status: created_chunk.upload_status,
            created_at: created_chunk.created_at,
            updated_at: created_chunk.updated_at,
        };

        Ok(created_chunk)
    }

    async fn find_by_reservation_and_index(
        &self,
        reservation_id: i64,
        chunk_index: i64,
    ) -> Result<Option<RoomChunkUpload>> {
        let chunk = sqlx::query_as!(
            RoomChunkUpload,
            r#"
            SELECT
                id,
                reservation_id,
                chunk_index,
                chunk_size,
                chunk_hash,
                upload_status as "upload_status: ChunkStatus",
                created_at,
                updated_at
            FROM room_chunk_uploads
            WHERE reservation_id = ? AND chunk_index = ?
            "#,
            reservation_id,
            chunk_index
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(chunk)
    }

    async fn count_by_reservation_id(&self, reservation_id: i64) -> Result<i64> {
        let count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM room_chunk_uploads
            WHERE reservation_id = ?
            "#,
            reservation_id
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(count.count)
    }

    async fn find_by_reservation_id(&self, reservation_id: i64) -> Result<Vec<RoomChunkUpload>> {
        let chunks = sqlx::query_as!(
            RoomChunkUpload,
            r#"
            SELECT
                id,
                reservation_id,
                chunk_index,
                chunk_size,
                chunk_hash,
                upload_status as "upload_status: ChunkStatus",
                created_at,
                updated_at
            FROM room_chunk_uploads
            WHERE reservation_id = ?
            ORDER BY chunk_index
            "#,
            reservation_id
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(chunks)
    }
}
