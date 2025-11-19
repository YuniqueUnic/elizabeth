use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow, postgres::PgRow, sqlite::SqliteRow};
use utoipa::ToSchema;

use crate::models::room::row_utils::read_datetime_from_any;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
#[sqlx(type_name = "INTEGER")]
#[repr(i64)]
pub enum ContentType {
    Text = 0,
    Image = 1,
    File = 2,
    Url = 3,
}

/// 数据库 RoomContent 模型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomContent {
    pub id: Option<i64>,
    pub room_id: i64,
    pub content_type: ContentType,
    pub text: Option<String>,      // The text content
    pub url: Option<String>,       // The URL to the content
    pub path: Option<String>, // The saved path to the content on server disk (UUID-based filename)
    pub file_name: Option<String>, // The original file name (for display and download)
    pub size: Option<i64>, // The size of the content, maybe the usize is better but the SQLite does not support u64
    pub mime_type: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

fn build_room_content_sqlite(row: &SqliteRow) -> Result<RoomContent, sqlx::Error> {
    Ok(RoomContent {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        content_type: row.try_get("content_type")?,
        text: row.try_get("text")?,
        url: row.try_get("url")?,
        path: row.try_get("path")?,
        file_name: row.try_get("file_name")?,
        size: row.try_get("size")?,
        mime_type: row.try_get("mime_type")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn build_room_content_pg(row: &PgRow) -> Result<RoomContent, sqlx::Error> {
    Ok(RoomContent {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        content_type: row.try_get("content_type")?,
        text: row.try_get("text")?,
        url: row.try_get("url")?,
        path: row.try_get("path")?,
        file_name: row.try_get("file_name")?,
        size: row.try_get("size")?,
        mime_type: row.try_get("mime_type")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn build_room_content_any(row: &AnyRow) -> Result<RoomContent, sqlx::Error> {
    Ok(RoomContent {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        content_type: row.try_get("content_type")?,
        text: row.try_get("text")?,
        url: row.try_get("url")?,
        path: row.try_get("path")?,
        file_name: row.try_get("file_name")?,
        size: row.try_get("size")?,
        mime_type: row.try_get("mime_type")?,
        created_at: read_datetime_from_any(row, "created_at")?,
        updated_at: read_datetime_from_any(row, "updated_at")?,
    })
}

impl<'r> FromRow<'r, SqliteRow> for RoomContent {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_room_content_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for RoomContent {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_room_content_pg(row)
    }
}

impl<'r> FromRow<'r, AnyRow> for RoomContent {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_room_content_any(row)
    }
}

#[bon::bon]
impl RoomContent {
    #[builder]
    pub fn builder(
        id: Option<i64>,
        room_id: i64,
        content_type: ContentType,
        now: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            room_id,
            content_type,
            created_at: now,
            updated_at: now,
            text: None,
            url: None,
            path: None,
            file_name: None,
            size: None,
            mime_type: None,
        }
    }

    /// Get timestamp for version control
    pub fn timestamp(&self) -> i64 {
        self.updated_at.and_utc().timestamp()
    }

    pub fn set_text(&mut self, text: String) {
        let size = string_storage_size(&text);
        self.text = Some(text);
        self.updated_at = Utc::now().naive_utc();
        self.mime_type = Some("text/plain".to_string());
        self.size = Some(size);
    }

    pub fn set_path(
        &mut self,
        path: String,
        content_type: ContentType,
        size: i64,
        mime_type: String,
    ) {
        self.path = Some(path);
        self.content_type = content_type;
        self.updated_at = Utc::now().naive_utc();
        self.mime_type = Some(mime_type);
        self.size = Some(size);
    }

    pub fn set_url(&mut self, url: String, mime_type: Option<String>) {
        let size = string_storage_size(&url);
        self.url = Some(url);
        self.content_type = ContentType::Url;
        self.updated_at = Utc::now().naive_utc();
        self.mime_type = mime_type;
        self.size = Some(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use sqlx::{Pool, Sqlite};

    #[tokio::test]
    async fn test_room_content_builder() -> Result<()> {
        // let pool = Pool::<Sqlite>::connect("sqlite::memory:").await?;
        let now = Utc::now().naive_utc();
        let mut content = RoomContent::builder()
            .id(1)
            .room_id(1)
            .content_type(ContentType::Text)
            .now(now)
            .build();
        content.set_text("hello world".to_string());
        println!("{:#?}", content.size);
        println!("{:#?}", content);
        Ok(())
    }

    #[test]
    fn set_text_records_utf8_byte_size() {
        let now = Utc::now().naive_utc();
        let mut content = RoomContent::builder()
            .id(1)
            .room_id(1)
            .content_type(ContentType::Text)
            .now(now)
            .build();

        content.set_text("你好世界".to_string());

        // "你好世界" 占用 12 个字节（每个汉字 3 个字节）
        assert_eq!(content.size, Some(12));
    }

    #[test]
    fn set_url_records_utf8_byte_size() {
        let now = Utc::now().naive_utc();
        let mut content = RoomContent::builder()
            .id(1)
            .room_id(1)
            .content_type(ContentType::Url)
            .now(now)
            .build();

        content.set_url(
            "https://例子。测试/路径".to_string(),
            Some("text/html".to_string()),
        );

        // Calculate UTF-8 byte length
        let expected = "https://例子。测试/路径".as_bytes().len() as i64;
        assert_eq!(content.size, Some(expected));
    }
}

fn string_storage_size(value: &str) -> i64 {
    value.len() as i64
}
