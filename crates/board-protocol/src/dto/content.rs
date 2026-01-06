use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::path::Path;
use utoipa::ToSchema;

use crate::models::UploadFileDescriptor;
use crate::models::content::{ContentType, RoomContent};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RoomContentView {
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub id: i64,
    pub content_type: ContentType,
    pub text: Option<String>,
    pub file_name: Option<String>,
    pub url: Option<String>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    pub size: Option<i64>,
    pub mime_type: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<RoomContent> for RoomContentView {
    fn from(value: RoomContent) -> Self {
        // 优先使用数据库 file_name 字段；若缺失，则从 path 回退提取
        let file_name = value.file_name.clone().or_else(|| {
            value.path.as_ref().and_then(|path| {
                Path::new(path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
            })
        });

        Self {
            id: value.id.unwrap_or_default(),
            content_type: value.content_type,
            text: value.text,
            file_name,
            url: value.url,
            size: value.size,
            mime_type: value.mime_type,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UploadContentResponse {
    pub uploaded: Vec<RoomContentView>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub current_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UploadPreparationRequest {
    pub files: Vec<UploadFileDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UploadPreparationResponse {
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub reservation_id: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub reserved_size: i64,
    pub expires_at: NaiveDateTime,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub current_size: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub remaining_size: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub max_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct DeleteContentRequest {
    #[cfg_attr(feature = "typescript-export", ts(type = "number[]"))]
    pub ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct DeleteContentResponse {
    #[cfg_attr(feature = "typescript-export", ts(type = "number[]"))]
    pub deleted: Vec<i64>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub freed_size: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub current_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UpdateContentRequest {
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub text: Option<String>,
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub url: Option<String>,
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UpdateContentResponse {
    pub updated: RoomContentView,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct CreateMessageRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct CreateMessageResponse {
    pub message: RoomContentView,
}
