use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Executor, Sqlite};
use std::sync::Arc;

use crate::{
    db::DbPool,
    models::content::{ContentType, RoomContent},
};

#[async_trait]
pub trait IRoomContentRepository: Send + Sync {
    async fn exists(&self, content_id: i64) -> Result<bool>;
    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent>;
    async fn find_by_id(&self, content_id: i64) -> Result<Option<RoomContent>>;
    async fn update(&self, room_content: &RoomContent) -> Result<RoomContent>;
    async fn delete(&self, room_name: &str) -> Result<bool>;
}

pub struct SqliteRoomContentRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomContentRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional_by_id<'e, E>(
        executor: E,
        content_id: i64,
    ) -> Result<Option<RoomContent>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let content = sqlx::query_as!(
            RoomContent,
            r#"
            SELECT
                id,
                room_id,
                content_type as "content_type: ContentType",
                text,
                url,
                path,
                size,
                mime_type,
                created_at,
                updated_at
            FROM room_contents
            WHERE id = ?
            "#,
            content_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(content)
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, content_id: i64) -> Result<RoomContent>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        Self::fetch_optional_by_id(executor, content_id)
            .await?
            .ok_or_else(|| anyhow!("room content not found for id {}", content_id))
    }
}

#[async_trait]
impl IRoomContentRepository for SqliteRoomContentRepository {
    async fn exists(&self, content_id: i64) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM room_contents WHERE id = ?)",
            content_id
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(exists != 0)
    }

    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let result = sqlx::query!(
            r#"
            INSERT INTO room_contents
                (room_id, content_type, text, url, path, size, mime_type, created_at, updated_at)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            room_content.room_id,
            room_content.content_type,
            room_content.text,
            room_content.url,
            room_content.path,
            room_content.size,
            room_content.mime_type,
            now,
            now
        )
        .execute(&mut *tx)
        .await?;

        let id = result.last_insert_rowid();
        let created_room_content = Self::fetch_by_id_or_err(&mut *tx, id).await?;

        tx.commit().await?;
        Ok(created_room_content)
    }

    async fn find_by_id(&self, content_id: i64) -> Result<Option<RoomContent>> {
        Self::fetch_optional_by_id(&*self.pool, content_id).await
    }

    async fn update(&self, room_content: &RoomContent) -> Result<RoomContent> {
        let content_id = room_content
            .id
            .ok_or_else(|| anyhow!("room content id is required for update"))?;
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        sqlx::query!(
            r#"
            UPDATE room_contents SET
                room_id = ?, content_type = ?, text = ?,
                url = ?, path = ?, size = ?, mime_type = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            room_content.room_id,
            room_content.content_type,
            room_content.text,
            room_content.url,
            room_content.path,
            room_content.size,
            room_content.mime_type,
            now,
            content_id
        )
        .execute(&mut *tx)
        .await?;

        let updated_room_content = Self::fetch_by_id_or_err(&mut *tx, content_id).await?;

        tx.commit().await?;
        Ok(updated_room_content)
    }

    async fn delete(&self, room_name: &str) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM room_contents WHERE room_id = (SELECT id FROM rooms WHERE name = ?)",
            room_name
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
