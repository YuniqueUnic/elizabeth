use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::{ChunkStatus, UploadFileDescriptor, UploadStatus};

/// 分块上传预留请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct ChunkedUploadPreparationRequest {
    pub files: Vec<UploadFileDescriptor>,
}

/// 分块上传预留响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct ChunkedUploadPreparationResponse {
    /// 预留 ID
    pub reservation_id: String,
    /// 上传令牌
    pub upload_token: String,
    /// 预留过期时间
    pub expires_at: NaiveDateTime,
    /// 文件清单
    pub files: Vec<ReservedFileInfo>,
}

/// 预留文件信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct ReservedFileInfo {
    /// 文件名
    pub name: String,
    /// 文件大小
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub size: i64,
    /// MIME 类型
    pub mime: Option<String>,
    /// 分块大小
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub chunk_size: i64,
    /// 总分块数
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub total_chunks: i64,
    /// 文件哈希
    pub file_hash: Option<String>,
}

/// 单个分块上传请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct ChunkUploadRequest {
    /// 上传令牌
    pub upload_token: String,
    /// 分块索引（从 0 开始）
    pub chunk_index: i32,
    /// 分块大小
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub chunk_size: i64,
    /// 分块哈希（可选，用于完整性验证）
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub chunk_hash: Option<String>,
}

/// 单个分块上传响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct ChunkUploadResponse {
    /// 分块索引
    pub chunk_index: i32,
    /// 分块大小
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub chunk_size: i64,
    /// 分块哈希
    pub chunk_hash: Option<String>,
    /// 上传状态
    pub upload_status: ChunkStatus,
    /// 上传时间
    pub uploaded_at: NaiveDateTime,
}

/// 上传状态查询请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UploadStatusQuery {
    /// 上传令牌（与 reservation_id 二选一）
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub upload_token: Option<String>,
    /// 预留 ID（与 upload_token 二选一）
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub reservation_id: Option<String>,
}

/// 单个分块状态信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct ChunkStatusInfo {
    /// 分块索引
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub chunk_index: i64,
    /// 分块大小
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub chunk_size: i64,
    /// 分块哈希
    pub chunk_hash: Option<String>,
    /// 上传状态
    pub upload_status: ChunkStatus,
    /// 上传时间
    pub uploaded_at: Option<NaiveDateTime>,
}

/// 上传状态查询响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UploadStatusResponse {
    /// 预留 ID
    pub reservation_id: String,
    /// 上传令牌
    pub upload_token: String,
    /// 上传状态
    pub upload_status: UploadStatus,
    /// 总分块数
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub total_chunks: i64,
    /// 已上传分块数
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub uploaded_chunks: i64,
    /// 上传进度百分比（0-100）
    pub progress_percentage: f64,
    /// 预留过期时间
    pub expires_at: NaiveDateTime,
    /// 已上传分块详细信息
    pub chunk_details: Vec<ChunkStatusInfo>,
    /// 预留大小
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub reserved_size: i64,
    /// 已上传大小
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub uploaded_size: i64,
    /// 是否超时
    pub is_expired: bool,
    /// 剩余时间（秒）
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    pub remaining_seconds: Option<i64>,
}

/// 文件合并完成请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct FileMergeRequest {
    pub reservation_id: String,
    pub final_hash: String,
}

/// 合并后的文件信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct MergedFileInfo {
    pub file_name: String,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub file_size: i64,
    pub file_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub content_id: Option<i64>,
}

/// 文件合并完成响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct FileMergeResponse {
    pub reservation_id: String,
    pub merged_files: Vec<MergedFileInfo>,
    pub message: String,
}
