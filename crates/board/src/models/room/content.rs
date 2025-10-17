use chrono::NaiveDateTime;
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
}

/// 数据库 RoomContent 模型，使用 FromRow 自动映射
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomContent {
    pub id: Option<i64>,
    pub room_id: i64,
    pub content_type: String,
    pub content_data: String,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub file_path: Option<String>,
    pub mime_type: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
