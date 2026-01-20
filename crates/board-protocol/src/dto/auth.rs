use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 登出请求结构（撤销访问令牌）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct LogoutRequest {
    /// 访问令牌
    pub access_token: String,
}

/// 清理响应结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct CleanupResponse {
    /// 清理的记录数量
    pub cleaned_records: u64,
    /// 操作结果消息
    pub message: String,
}
