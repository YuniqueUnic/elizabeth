use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
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

/// 数据库 RoomContent 模型，使用 FromRow 自动映射
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomContent {
    pub id: Option<i64>,
    pub room_id: i64,
    pub content_type: ContentType,
    pub text: Option<String>, // the text content
    pub url: Option<String>,  // the url to the content
    pub path: Option<String>, // the saved path to the content on server disk
    pub size: Option<i64>, // the size of the content, maybe the usize is better but the sqlite does not support u64
    pub mime_type: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
            size: None,
            mime_type: None,
        }
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

        // 计算 UTF-8 字节长度
        let expected = "https://例子。测试/路径".as_bytes().len() as i64;
        assert_eq!(content.size, Some(expected));
    }
}

fn string_storage_size(value: &str) -> i64 {
    value.len() as i64
}
