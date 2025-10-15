use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

// 自定义 DateTime 类型，用于 OpenAPI 文档
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NativeDateTimeWrapper(String);

impl From<NaiveDateTime> for NativeDateTimeWrapper {
    fn from(dt: NaiveDateTime) -> Self {
        NativeDateTimeWrapper(dt.format("%Y-%m-%d %H:%M:%S").to_string())
    }
}

impl From<NativeDateTimeWrapper> for NaiveDateTime {
    fn from(dt: NativeDateTimeWrapper) -> Self {
        NaiveDateTime::parse_from_str(&dt.0, "%Y-%m-%d %H:%M:%S").unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub enum RoomStatus {
    #[serde(rename = "open")]
    #[default]
    Open,
    #[serde(rename = "lock")]
    Lock,
    #[serde(rename = "close")]
    Close,
}

impl From<i64> for RoomStatus {
    fn from(value: i64) -> Self {
        match value {
            0 => RoomStatus::Open,
            1 => RoomStatus::Lock,
            2 => RoomStatus::Close,
            _ => RoomStatus::Open,
        }
    }
}

impl From<RoomStatus> for i64 {
    fn from(status: RoomStatus) -> Self {
        match status {
            RoomStatus::Open => 0,
            RoomStatus::Lock => 1,
            RoomStatus::Close => 2,
        }
    }
}

/// 数据库 Room 模型，使用 FromRow 自动映射

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Room {
    pub id: Option<i64>,
    pub name: String,
    pub password: Option<String>,
    pub status: i64,
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

/// API 响应 Room 模型，使用 CustomDateTime 用于 OpenAPI
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomResponse {
    pub id: Option<i64>,
    pub name: String,
    pub password: Option<String>,
    pub status: i64,
    pub max_size: i64,
    pub current_size: i64,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
    pub expire_at: Option<NativeDateTimeWrapper>,
    pub created_at: NativeDateTimeWrapper,
    pub updated_at: NativeDateTimeWrapper,
    pub allow_edit: bool,
    pub allow_download: bool,
    pub allow_preview: bool,
}

impl From<Room> for RoomResponse {
    fn from(room: Room) -> Self {
        Self {
            id: room.id,
            name: room.name,
            password: room.password,
            status: room.status,
            max_size: room.max_size,
            current_size: room.current_size,
            max_times_entered: room.max_times_entered,
            current_times_entered: room.current_times_entered,
            expire_at: room.expire_at.map(NativeDateTimeWrapper::from),
            created_at: NativeDateTimeWrapper::from(room.created_at),
            updated_at: NativeDateTimeWrapper::from(room.updated_at),
            allow_edit: room.allow_edit,
            allow_download: room.allow_download,
            allow_preview: room.allow_preview,
        }
    }
}

impl Room {
    pub fn new(name: String, password: Option<String>) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id: None,
            name,
            password,
            status: i64::from(RoomStatus::default()),
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

    pub fn status_enum(&self) -> RoomStatus {
        RoomStatus::from(self.status)
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
            && self.status_enum() != RoomStatus::Close
            && self.current_times_entered < self.max_times_entered
    }

    #[allow(unused)]
    pub fn can_add_content(&self, content_size: i64) -> bool {
        self.allow_edit && self.current_size + content_size <= self.max_size
    }
}
