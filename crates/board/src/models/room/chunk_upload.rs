use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// 分块状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum ChunkStatus {
    #[default]
    Pending, // 等待上传
    Uploaded, // 已上传
    Verified, // 已验证
    Failed,   // 失败
}

impl std::fmt::Display for ChunkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkStatus::Pending => write!(f, "pending"),
            ChunkStatus::Uploaded => write!(f, "uploaded"),
            ChunkStatus::Verified => write!(f, "verified"),
            ChunkStatus::Failed => write!(f, "failed"),
        }
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
            std::borrow::Cow::Owned(self.to_string()),
        ));
        Ok(sqlx::encode::IsNull::No)
    }
}

/// 房间分块上传记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomChunkUpload {
    pub id: Option<i64>,
    pub reservation_id: i64,        // 关联的预留 ID
    pub chunk_index: i64,           // 分块索引（从 0 开始）
    pub chunk_size: i64,            // 分块大小
    pub chunk_hash: Option<String>, // 分块哈希值
    pub upload_status: ChunkStatus, // 分块状态
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// 分块上传请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkUploadRequest {
    pub chunk_index: i64,
    pub chunk_hash: String,
}

/// 分块上传响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkUploadResponse {
    pub chunk_index: i64,
    pub upload_status: ChunkStatus,
    pub uploaded_chunks: i64,
    pub total_chunks: i64,
    pub upload_progress: f64,
}

/// 分块状态信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkStatusInfo {
    pub chunk_index: i64,
    pub status: ChunkStatus,
    pub chunk_hash: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// 上传状态查询响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkedUploadStatusResponse {
    pub reservation_id: i64,
    pub upload_status: String,
    pub total_chunks: i64,
    pub uploaded_chunks: i64,
    pub upload_progress: f64,
    pub chunk_status: Vec<ChunkStatusInfo>,
}

/// 文件合并完成请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileMergeRequest {
    pub reservation_id: String,
    pub final_hash: String,
}

/// 文件合并完成响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileMergeResponse {
    pub reservation_id: String,
    pub merged_files: Vec<MergedFileInfo>,
    pub message: String,
}

/// 合并后的文件信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergedFileInfo {
    pub file_name: String,
    pub file_size: i64,
    pub file_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_id: Option<i64>,
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

impl ChunkStatusInfo {
    /// 从 RoomChunkUpload 创建状态信息
    pub fn from_chunk_upload(chunk: &RoomChunkUpload) -> Self {
        Self {
            chunk_index: chunk.chunk_index,
            status: chunk.upload_status.clone(),
            chunk_hash: chunk.chunk_hash.clone(),
            created_at: chunk.created_at,
            updated_at: chunk.updated_at,
        }
    }
}
