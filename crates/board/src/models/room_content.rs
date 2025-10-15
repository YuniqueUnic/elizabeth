use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use super::NativeDateTimeWrapper;

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ContentType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "file")]
    File,
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AccessAction {
    #[serde(rename = "enter")]
    Enter,
    #[serde(rename = "exit")]
    Exit,
    #[serde(rename = "create_content")]
    CreateContent,
    #[serde(rename = "delete_content")]
    DeleteContent,
}

#[allow(unused)]
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
    pub created_at: NativeDateTimeWrapper,
    pub updated_at: NativeDateTimeWrapper,
}

#[allow(unused)]
/// 数据库 RoomAccessLog 模型，使用 FromRow 自动映射
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomAccessLog {
    pub id: Option<i64>,
    pub room_id: i64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub access_time: NativeDateTimeWrapper,
    pub action: String,
    pub details: Option<String>,
}
