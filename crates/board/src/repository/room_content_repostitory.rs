use anyhow::Result;
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use std::sync::Arc;

use crate::{
    db::DbPool,
    models::content::{ContentType, RoomContent},
};

// 我希望构建一个用于存储 room content 的 repository
// 然后这两个表通过 room id 进行联合
// room content 有三种类型
// - 纯文本 text
// - 用户上传文件，存储到我们提供的 server/s3 上，path
// - 用户直接传入的 url, 我们只负责存储 url 文本，然后这样的 url, 我们会尝试提供前端的预览，url

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
}

#[async_trait]
impl IRoomContentRepository for SqliteRoomContentRepository {
    async fn exists(&self, content_id: i64) -> Result<bool> {
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM room_contents WHERE id = ?",
            content_id
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(count > 0)
    }
    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query!(
            "INSERT INTO room_contents (room_id, content_type, text, url, path, size, mime_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .execute(&*self.pool)
        .await?;

        let id = result.last_insert_rowid();

        let created_room_content = self.find_by_id(id).await?;

        created_room_content.ok_or_else(|| anyhow::anyhow!("failed to get created room_content"))
    }
    async fn find_by_id(&self, content_id: i64) -> Result<Option<RoomContent>> {
        let room_content = sqlx::query_as!(
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
        .fetch_optional(&*self.pool)
        .await?;

        Ok(room_content)
    }
    async fn update(&self, room_content: &RoomContent) -> Result<RoomContent> {
        if room_content.id.is_none() {
            return Err(anyhow::anyhow!("Room content id is None"));
        }

        let now = Utc::now().naive_utc();
        let room_content_id = room_content.id.unwrap();
        let result = sqlx::query!(
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
            room_content_id
        )
        .execute(&*self.pool)
        .await?;

        let updated_room_content = self.find_by_id(room_content_id).await?;

        updated_room_content.ok_or_else(|| anyhow::anyhow!("failed to get updated room_content"))
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
