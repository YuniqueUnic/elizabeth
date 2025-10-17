use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "action")]
#[sqlx(type_name = "INTEGER")]
#[repr(i64)]
pub enum AccessAction {
    Enter = 0,
    Exit = 1,
    CreateContent = 2,
    DeleteContent = 3,
}

/// 数据库 RoomAccessLog 模型，使用 FromRow 自动映射
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomAccessLog {
    pub id: Option<i64>,
    pub room_id: i64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub access_time: NaiveDateTime,
    pub action: String,
    pub details: Option<String>,
}
