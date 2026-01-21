use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Any;
use std::sync::Arc;

use crate::models::room::row_utils::format_naive_datetime;
use crate::{
    db::DbPool,
    models::content::{ContentType, RoomContent},
};

const CONTENT_SELECT_BASE: &str = r#"
    SELECT
        id,
        room_id,
        content_type,
        text,
        url,
        path,
        file_name,
        size,
        mime_type,
        CAST(created_at AS TEXT) as created_at,
        CAST(updated_at AS TEXT) as updated_at
    FROM room_contents
"#;

#[async_trait]
pub trait IRoomContentRepository: Send + Sync {
    async fn exists(&self, content_id: i64) -> Result<bool>;
    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent>;
    async fn find_by_id(&self, content_id: i64) -> Result<Option<RoomContent>>;
    async fn update(&self, room_content: &RoomContent) -> Result<RoomContent>;
    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomContent>>;
    async fn delete_by_ids(&self, room_id: i64, content_ids: &[i64]) -> Result<u64>;
    async fn delete_by_room_id(&self, room_id: i64) -> Result<u64>;
    async fn total_size_by_room(&self, room_id: i64) -> Result<i64>;
    async fn delete(&self, room_name: &str) -> Result<bool>;
}

pub struct RoomContentRepository {
    pool: Arc<DbPool>,
}

impl RoomContentRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional_by_id<'e, E>(
        executor: E,
        content_id: i64,
    ) -> Result<Option<RoomContent>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!("{CONTENT_SELECT_BASE} WHERE id = $1");
        let content = sqlx::query_as::<_, RoomContent>(&sql)
            .bind(content_id)
            .fetch_optional(executor)
            .await?;
        Ok(content)
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, content_id: i64) -> Result<RoomContent>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        Self::fetch_optional_by_id(executor, content_id)
            .await?
            .ok_or_else(|| anyhow!("room content not found for id {}", content_id))
    }
}

#[async_trait]
impl IRoomContentRepository for RoomContentRepository {
    async fn exists(&self, content_id: i64) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT CASE WHEN EXISTS(SELECT 1 FROM room_contents WHERE id = $1) THEN 1 ELSE 0 END",
        )
        .bind(content_id)
        .fetch_one(&*self.pool)
        .await?;

        Ok(exists != 0)
    }

    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let now_str = format_naive_datetime(now);
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_contents
                (room_id, content_type, text, url, path, file_name, size, mime_type, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(room_content.room_id)
        .bind(room_content.content_type)
        .bind(&room_content.text)
        .bind(&room_content.url)
        .bind(&room_content.path)
        .bind(&room_content.file_name)
        .bind(room_content.size)
        .bind(&room_content.mime_type)
        .bind(now_str.clone())
        .bind(now_str)
        .fetch_one(&mut *tx)
        .await?;

        let created = Self::fetch_by_id_or_err(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(created)
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
        let now_str = format_naive_datetime(now);
        sqlx::query(
            r#"
            UPDATE room_contents SET
                room_id = $1, content_type = $2, text = $3,
                url = $4, path = $5, file_name = $6, size = $7, mime_type = $8,
                updated_at = $9
            WHERE id = $10
            "#,
        )
        .bind(room_content.room_id)
        .bind(room_content.content_type)
        .bind(&room_content.text)
        .bind(&room_content.url)
        .bind(&room_content.path)
        .bind(&room_content.file_name)
        .bind(room_content.size)
        .bind(&room_content.mime_type)
        .bind(now_str)
        .bind(content_id)
        .execute(&mut *tx)
        .await?;

        let updated = Self::fetch_by_id_or_err(&mut *tx, content_id).await?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomContent>> {
        let sql = format!("{CONTENT_SELECT_BASE} WHERE room_id = $1 ORDER BY created_at DESC");
        let rows = sqlx::query_as::<_, RoomContent>(&sql)
            .bind(room_id)
            .fetch_all(&*self.pool)
            .await?;
        Ok(rows)
    }

    async fn delete_by_ids(&self, room_id: i64, content_ids: &[i64]) -> Result<u64> {
        if content_ids.is_empty() {
            return Ok(0);
        }

        let mut sql = String::from("DELETE FROM room_contents WHERE room_id = $1 AND id IN (");
        for i in 0..content_ids.len() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push('$');
            sql.push_str(&(i + 2).to_string());
        }
        sql.push(')');

        let mut query = sqlx::query(&sql).bind(room_id);
        for id in content_ids {
            query = query.bind(id);
        }
        let result = query.execute(&*self.pool).await?;
        Ok(result.rows_affected())
    }

    async fn delete_by_room_id(&self, room_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM room_contents WHERE room_id = $1")
            .bind(room_id)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn total_size_by_room(&self, room_id: i64) -> Result<i64> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(size), 0) FROM room_contents WHERE room_id = $1",
        )
        .bind(room_id)
        .fetch_one(&*self.pool)
        .await?;

        Ok(total)
    }

    async fn delete(&self, room_name: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM room_contents WHERE room_id = (SELECT id FROM rooms WHERE name = $1)",
        )
        .bind(room_name)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
