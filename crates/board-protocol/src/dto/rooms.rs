use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::token::RoomTokenClaims;
use crate::models::RoomToken;

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct IssueTokenRequest {
    /// 房间密码，如果房间设置了密码，则必须填写
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub password: Option<String>,
    /// 已有的房间 token，可用于在无需密码的情况下续签
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub token: Option<String>,
    /// 是否请求刷新令牌对
    #[serde(default)]
    pub with_refresh_token: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct IssueTokenResponse {
    pub token: String,
    pub claims: RoomTokenClaims,
    pub expires_at: NaiveDateTime,
    /// 刷新令牌（仅在请求时返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub refresh_token: Option<String>,
    /// 刷新令牌过期时间（仅在请求时返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub refresh_token_expires_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct ValidateTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct ValidateTokenResponse {
    pub claims: RoomTokenClaims,
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UpdateRoomPermissionRequest {
    #[serde(default)]
    pub edit: bool,
    #[serde(default)]
    pub share: bool,
    #[serde(default)]
    pub delete: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UpdateRoomSettingsRequest {
    /// 房间密码（可选，设置为 None 表示移除密码）
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub password: Option<String>,
    /// 房间过期时间（可选，设置为 None 表示永不过期）
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub expire_at: Option<NaiveDateTime>,
    /// 最大进入次数（可选）
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub max_times_entered: Option<i64>,
    /// 最大容量限制（可选，单位：字节）
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub max_size: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RevokeTokenResponse {
    pub revoked: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct DeleteRoomResponse {
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RoomTokenView {
    pub jti: String,
    pub expires_at: NaiveDateTime,
    pub revoked_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

impl From<RoomToken> for RoomTokenView {
    fn from(value: RoomToken) -> Self {
        Self {
            jti: value.jti,
            expires_at: value.expires_at,
            revoked_at: value.revoked_at,
            created_at: value.created_at,
        }
    }
}
