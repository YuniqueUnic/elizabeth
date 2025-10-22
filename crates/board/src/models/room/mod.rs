use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::models::permission::RoomPermission;

pub mod content;
pub mod metric;
pub mod permission;
pub mod refresh_token;
pub mod token;
pub mod upload_reservation;

pub use refresh_token::{
    CreateRefreshTokenRequest, RefreshTokenRequest, RefreshTokenResponse, RoomRefreshToken,
    TokenBlacklistEntry,
};
pub use token::RoomToken;
pub use upload_reservation::{RoomUploadReservation, UploadFileDescriptor};

pub const DEFAULT_MAX_TIMES_ENTER_ROOM: i64 = 100;
pub const DEFAULT_MAX_ROOM_CONTENT_SIZE: i64 = 10 * 1024 * 1024;

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
    pub slug: String,
    pub password: Option<String>,
    pub status: RoomStatus,
    pub max_size: i64,
    pub current_size: i64,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
    pub expire_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub permission: RoomPermission,
}

impl Room {
    pub fn new(name: String, password: Option<String>) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id: None,
            slug: name.clone(),
            name,
            password,
            status: RoomStatus::default(),
            max_size: DEFAULT_MAX_ROOM_CONTENT_SIZE, // 10MB
            current_size: 0,
            max_times_entered: DEFAULT_MAX_TIMES_ENTER_ROOM,
            current_times_entered: 0,
            expire_at: None,
            created_at: now,
            updated_at: now,
            permission: RoomPermission::new().with_all(),
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
        self.permission.can_edit() && self.current_size + content_size <= self.max_size
    }
}
