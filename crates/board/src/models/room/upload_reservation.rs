use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::ToSchema;

/// 上传状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum UploadStatus {
    #[default]
    Pending, // 等待上传
    Uploading, // 上传中
    Completed, // 已完成
    Failed,    // 失败
    Expired,   // 已过期
}

impl std::fmt::Display for UploadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_storage_value())
    }
}

impl UploadStatus {
    fn as_storage_value(&self) -> &'static str {
        match self {
            UploadStatus::Pending => "pending",
            UploadStatus::Uploading => "uploading",
            UploadStatus::Completed => "completed",
            UploadStatus::Failed => "failed",
            UploadStatus::Expired => "expired",
        }
    }
}

impl std::str::FromStr for UploadStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(UploadStatus::Pending),
            "uploading" => Ok(UploadStatus::Uploading),
            "completed" => Ok(UploadStatus::Completed),
            "failed" => Ok(UploadStatus::Failed),
            "expired" => Ok(UploadStatus::Expired),
            _ => Err(format!("Invalid upload status: {}", s)),
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for UploadStatus {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for UploadStatus {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for UploadStatus {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        args.push(sqlx::sqlite::SqliteArgumentValue::Text(
            std::borrow::Cow::Borrowed(self.as_storage_value()),
        ));
        Ok(sqlx::encode::IsNull::No)
    }
}

impl sqlx::Type<sqlx::Postgres> for UploadStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for UploadStatus {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for UploadStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <&str as sqlx::Encode<sqlx::Postgres>>::encode(self.as_storage_value(), buf)
    }
}

/// 客户端上报的文件信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UploadFileDescriptor {
    pub name: String,
    pub size: i64,
    pub mime: Option<String>,
    // 分块上传相关字段
    pub chunk_size: Option<i32>,
    pub file_hash: Option<String>,
}

/// 分块上传预留请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkedUploadPreparationRequest {
    pub files: Vec<UploadFileDescriptor>,
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

    // 分块上传相关字段
    pub chunked_upload: Option<bool>,        // 是否为分块上传
    pub total_chunks: Option<i64>,           // 总分块数
    pub uploaded_chunks: Option<i64>,        // 已上传分块数
    pub file_hash: Option<String>,           // 文件完整哈希
    pub chunk_size: Option<i64>,             // 分块大小
    pub upload_status: Option<UploadStatus>, // 上传状态
}

impl RoomUploadReservation {
    pub fn is_expired(&self, now: NaiveDateTime) -> bool {
        self.consumed_at.is_none() && now >= self.expires_at
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed_at.is_some()
    }

    /// 检查是否为分块上传
    pub fn is_chunked_upload(&self) -> bool {
        self.chunked_upload.unwrap_or(false)
    }

    /// 获取上传进度百分比
    pub fn upload_progress(&self) -> f64 {
        if !self.is_chunked_upload() {
            return if self.is_consumed() { 100.0 } else { 0.0 };
        }

        match (self.total_chunks, self.uploaded_chunks) {
            (Some(total), Some(uploaded)) => {
                if total == 0 {
                    0.0
                } else {
                    (uploaded as f64 / total as f64) * 100.0
                }
            }
            _ => 0.0,
        }
    }

    /// 检查上传是否完成
    pub fn is_upload_completed(&self) -> bool {
        matches!(self.upload_status, Some(UploadStatus::Completed))
    }

    /// 检查上传是否失败
    pub fn is_upload_failed(&self) -> bool {
        matches!(self.upload_status, Some(UploadStatus::Failed))
    }

    /// 检查是否可以继续上传
    pub fn can_continue_upload(&self, now: NaiveDateTime) -> bool {
        !self.is_expired(now)
            && !self.is_consumed()
            && !self.is_upload_failed()
            && !matches!(self.upload_status, Some(UploadStatus::Expired))
    }

    /// 获取剩余可上传分块数
    pub fn remaining_chunks(&self) -> Option<i64> {
        if !self.is_chunked_upload() {
            return None;
        }

        match (self.total_chunks, self.uploaded_chunks) {
            (Some(total), Some(uploaded)) => Some(total - uploaded),
            _ => self.total_chunks,
        }
    }
}
