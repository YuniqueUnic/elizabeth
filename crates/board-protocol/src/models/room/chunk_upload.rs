use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow, postgres::PgRow, sqlite::SqliteRow};

use crate::models::room::row_utils::read_datetime_from_any;
use utoipa::ToSchema;

/// 分块状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub enum ChunkStatus {
    #[default]
    Pending, // 等待上传
    Uploaded, // 已上传
    Verified, // 已验证
    Failed,   // 失败
}

impl ChunkStatus {
    fn as_storage_value(&self) -> &'static str {
        match self {
            ChunkStatus::Pending => "pending",
            ChunkStatus::Uploaded => "uploaded",
            ChunkStatus::Verified => "verified",
            ChunkStatus::Failed => "failed",
        }
    }
}

impl std::fmt::Display for ChunkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_storage_value())
    }
}

impl std::str::FromStr for ChunkStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(ChunkStatus::Pending),
            "uploaded" => Ok(ChunkStatus::Uploaded),
            "verified" => Ok(ChunkStatus::Verified),
            "failed" => Ok(ChunkStatus::Failed),
            _ => Err(format!("Invalid chunk status: {}", s)),
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for ChunkStatus {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ChunkStatus {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ChunkStatus {
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

impl sqlx::Type<sqlx::Postgres> for ChunkStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for ChunkStatus {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for ChunkStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <&str as sqlx::Encode<sqlx::Postgres>>::encode(self.as_storage_value(), buf)
    }
}

impl sqlx::Type<sqlx::Any> for ChunkStatus {
    fn type_info() -> <sqlx::Any as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Any>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Any> for ChunkStatus {
    fn decode(value: sqlx::any::AnyValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Any>>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Any> for ChunkStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Any as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <&str as sqlx::Encode<sqlx::Any>>::encode(self.as_storage_value(), buf)
    }
}

/// 房间分块上传记录
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RoomChunkUpload {
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    pub id: Option<i64>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub reservation_id: i64, // 关联的预留 ID
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub chunk_index: i64, // 分块索引（从 0 开始）
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub chunk_size: i64, // 分块大小
    pub chunk_hash: Option<String>, // 分块哈希值
    pub upload_status: ChunkStatus, // 分块状态
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl RoomChunkUpload {
    /// 创建新的分块上传记录
    pub fn new(
        reservation_id: i64,
        chunk_index: i64,
        chunk_size: i64,
        chunk_hash: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: None,
            reservation_id,
            chunk_index,
            chunk_size,
            chunk_hash,
            upload_status: ChunkStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    /// 标记为已上传
    pub fn mark_as_uploaded(&mut self, chunk_hash: String) {
        self.chunk_hash = Some(chunk_hash);
        self.upload_status = ChunkStatus::Uploaded;
        self.updated_at = chrono::Utc::now().naive_utc();
    }

    /// 标记为已验证
    pub fn mark_as_verified(&mut self) {
        self.upload_status = ChunkStatus::Verified;
        self.updated_at = chrono::Utc::now().naive_utc();
    }

    /// 标记为失败
    pub fn mark_as_failed(&mut self) {
        self.upload_status = ChunkStatus::Failed;
        self.updated_at = chrono::Utc::now().naive_utc();
    }

    /// 检查是否已上传
    pub fn is_uploaded(&self) -> bool {
        matches!(
            self.upload_status,
            ChunkStatus::Uploaded | ChunkStatus::Verified
        )
    }

    /// 检查是否已验证
    pub fn is_verified(&self) -> bool {
        matches!(self.upload_status, ChunkStatus::Verified)
    }

    /// 检查是否失败
    pub fn is_failed(&self) -> bool {
        matches!(self.upload_status, ChunkStatus::Failed)
    }

    /// 检查是否等待上传
    pub fn is_pending(&self) -> bool {
        matches!(self.upload_status, ChunkStatus::Pending)
    }

    /// 验证分块哈希
    pub fn verify_hash(&self, expected_hash: &str) -> bool {
        match &self.chunk_hash {
            Some(hash) => hash == expected_hash,
            None => false,
        }
    }
}

fn build_chunk_from_sqlite(row: &SqliteRow) -> Result<RoomChunkUpload, sqlx::Error> {
    Ok(RoomChunkUpload {
        id: row.try_get("id")?,
        reservation_id: row.try_get("reservation_id")?,
        chunk_index: row.try_get("chunk_index")?,
        chunk_size: row.try_get("chunk_size")?,
        chunk_hash: row.try_get("chunk_hash")?,
        upload_status: row.try_get("upload_status")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn build_chunk_from_pg(row: &PgRow) -> Result<RoomChunkUpload, sqlx::Error> {
    Ok(RoomChunkUpload {
        id: row.try_get("id")?,
        reservation_id: row.try_get("reservation_id")?,
        chunk_index: row.try_get("chunk_index")?,
        chunk_size: row.try_get("chunk_size")?,
        chunk_hash: row.try_get("chunk_hash")?,
        upload_status: row.try_get("upload_status")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn build_chunk_from_any(row: &AnyRow) -> Result<RoomChunkUpload, sqlx::Error> {
    Ok(RoomChunkUpload {
        id: row.try_get("id")?,
        reservation_id: row.try_get("reservation_id")?,
        chunk_index: row.try_get("chunk_index")?,
        chunk_size: row.try_get("chunk_size")?,
        chunk_hash: row.try_get("chunk_hash")?,
        upload_status: row.try_get("upload_status")?,
        created_at: read_datetime_from_any(row, "created_at")?,
        updated_at: read_datetime_from_any(row, "updated_at")?,
    })
}

impl<'r> FromRow<'r, SqliteRow> for RoomChunkUpload {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_chunk_from_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for RoomChunkUpload {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_chunk_from_pg(row)
    }
}

impl<'r> FromRow<'r, AnyRow> for RoomChunkUpload {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_chunk_from_any(row)
    }
}
