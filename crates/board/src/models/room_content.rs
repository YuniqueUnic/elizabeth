use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "INTEGER")]
#[repr(i64)]
pub enum ContentType {
    Text = 0,
    Image = 1,
    File = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "action")]
#[sqlx(type_name = "INTEGER")]
#[repr(i64)]
pub enum AccessAction {
    #[serde(rename = "enter")]
    Enter = 0,
    #[serde(rename = "exit")]
    Exit = 1,
    #[serde(rename = "create_content")]
    CreateContent = 2,
    #[serde(rename = "delete_content")]
    DeleteContent = 3,
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

/// 数据库 RoomAccessLog 模型，使用 FromRow 自动映射
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomAccessLog {
    pub id: Option<i64>,
    pub room_id: i64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    // #[schema(value_type = String, format = DateTime)]
    pub access_time: NaiveDateTime,
    pub action: String,
    pub details: Option<String>,
}
