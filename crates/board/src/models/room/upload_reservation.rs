use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// 客户端上报的文件信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UploadFileDescriptor {
    pub name: String,
    pub size: i64,
    pub mime: Option<String>,
}

/// 上传预留记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomUploadReservation {
    pub id: Option<i64>,
    pub room_id: i64,
    pub token_jti: String,
    pub file_manifest: String,
    pub reserved_size: i64,
    pub reserved_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub consumed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl RoomUploadReservation {
    pub fn is_expired(&self, now: NaiveDateTime) -> bool {
        self.consumed_at.is_none() && now >= self.expires_at
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed_at.is_some()
    }
}
