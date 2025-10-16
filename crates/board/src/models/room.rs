use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default, sqlx::Type,
)] // 如果使用 sqlx
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "INTEGER")]
#[repr(i64)]
pub enum RoomStatus {
    #[default]
    Open = 0,
    Lock = 1,
    Close = 2,
}

/// 数据库与 API Room 模型，使用 FromRow 自动映射
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Room {
    pub id: Option<i64>,
    pub name: String,
    pub password: Option<String>,
    pub status: RoomStatus,
    pub max_size: i64,
    pub current_size: i64,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
    pub expire_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub allow_edit: bool,
    pub allow_download: bool,
    pub allow_preview: bool,
}

impl Room {
    pub fn new(name: String, password: Option<String>) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id: None,
            name,
            password,
            status: RoomStatus::default(),
            max_size: 10 * 1024 * 1024, // 10MB
            current_size: 0,
            max_times_entered: 100,
            current_times_entered: 0,
            expire_at: None,
            created_at: now,
            updated_at: now,
            allow_edit: true,
            allow_download: true,
            allow_preview: true,
        }
    }

    pub fn status(&self) -> RoomStatus {
        self.status
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expire_at) = self.expire_at {
            Utc::now().naive_utc() > expire_at
        } else {
            false
        }
    }

    pub fn can_enter(&self) -> bool {
        !self.is_expired()
            && self.status() != RoomStatus::Close
            && self.current_times_entered < self.max_times_entered
    }

    pub fn can_add_content(&self, content_size: i64) -> bool {
        self.allow_edit && self.current_size + content_size <= self.max_size
    }
}
