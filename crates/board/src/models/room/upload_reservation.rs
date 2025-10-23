use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::ToSchema;

/// 上传状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema, PartialEq)]
#[sqlx(type_name = "text")]
#[derive(Default)]
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
        match self {
            UploadStatus::Pending => write!(f, "pending"),
            UploadStatus::Uploading => write!(f, "uploading"),
            UploadStatus::Completed => write!(f, "completed"),
            UploadStatus::Failed => write!(f, "failed"),
            UploadStatus::Expired => write!(f, "expired"),
        }
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
