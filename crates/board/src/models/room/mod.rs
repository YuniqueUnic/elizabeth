use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow, postgres::PgRow, sqlite::SqliteRow};
use utoipa::ToSchema;

use crate::constants::room::{DEFAULT_MAX_ROOM_CONTENT_SIZE, DEFAULT_MAX_TIMES_ENTER_ROOM};
use crate::models::permission::RoomPermission;
use crate::models::room::row_utils::{read_datetime_from_any, read_optional_datetime_from_any};

pub mod chunk_upload;
pub mod content;
pub mod permission;
pub mod refresh_token;
pub mod row_utils;
pub mod token;
pub mod upload_reservation;

pub use chunk_upload::{
    ChunkStatus, ChunkStatusInfo, ChunkUploadRequest, ChunkUploadResponse,
    ChunkedUploadStatusResponse, FileMergeRequest, FileMergeResponse, MergedFileInfo,
    RoomChunkUpload,
};
pub use refresh_token::{
    CreateRefreshTokenRequest, RefreshTokenRequest, RefreshTokenResponse, RoomRefreshToken,
    TokenBlacklistEntry,
};
pub use token::RoomToken;
pub use upload_reservation::{RoomUploadReservation, UploadFileDescriptor, UploadStatus};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default, sqlx::Type,
)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "INTEGER")]
#[repr(i64)]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub enum RoomStatus {
    #[default]
    Open = 0,
    Lock = 1,
    Close = 2,
}

/// 数据库与 API Room 模型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript-export", ts(export))]
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

fn build_room_from_sqlite(row: &SqliteRow) -> Result<Room, sqlx::Error> {
    Ok(Room {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        slug: row.try_get("slug")?,
        password: row.try_get("password")?,
        status: row.try_get("status")?,
        max_size: row.try_get("max_size")?,
        current_size: row.try_get("current_size")?,
        max_times_entered: row.try_get("max_times_entered")?,
        current_times_entered: row.try_get("current_times_entered")?,
        expire_at: row.try_get("expire_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        permission: row.try_get("permission")?,
    })
}

fn build_room_from_pg(row: &PgRow) -> Result<Room, sqlx::Error> {
    Ok(Room {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        slug: row.try_get("slug")?,
        password: row.try_get("password")?,
        status: row.try_get("status")?,
        max_size: row.try_get("max_size")?,
        current_size: row.try_get("current_size")?,
        max_times_entered: row.try_get("max_times_entered")?,
        current_times_entered: row.try_get("current_times_entered")?,
        expire_at: row.try_get("expire_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        permission: row.try_get("permission")?,
    })
}

fn build_room_from_any(row: &AnyRow) -> Result<Room, sqlx::Error> {
    let permission_bits: i64 = row.try_get("permission")?;
    let permission = RoomPermission::from_bits(permission_bits as u8).unwrap_or_default();
    Ok(Room {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        slug: row.try_get("slug")?,
        password: row.try_get("password")?,
        status: row.try_get("status")?,
        max_size: row.try_get("max_size")?,
        current_size: row.try_get("current_size")?,
        max_times_entered: row.try_get("max_times_entered")?,
        current_times_entered: row.try_get("current_times_entered")?,
        expire_at: read_optional_datetime_from_any(row, "expire_at")?,
        created_at: read_datetime_from_any(row, "created_at")?,
        updated_at: read_datetime_from_any(row, "updated_at")?,
        permission,
    })
}

impl<'r> FromRow<'r, SqliteRow> for Room {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_room_from_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for Room {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_room_from_pg(row)
    }
}

impl<'r> FromRow<'r, AnyRow> for Room {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_room_from_any(row)
    }
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
            max_size: DEFAULT_MAX_ROOM_CONTENT_SIZE, // 10 megabytes
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
