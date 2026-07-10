use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::token::RoomTokenClaims;
use crate::models::{Room, RoomStatus, RoomToken, permission::RoomPermission};

#[derive(Debug, Default, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct CreateRoomRequest {
    /// 可选房间密码。密码只在请求边界出现，不会在房间响应中回显。
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RoomView {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub status: RoomStatus,
    pub max_size: i64,
    pub current_size: i64,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
    pub expire_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    #[cfg_attr(feature = "typescript-export", schemars(with = "u8"))]
    pub permission: RoomPermission,
    pub password_protected: bool,
}

impl From<&Room> for RoomView {
    fn from(room: &Room) -> Self {
        Self {
            id: room.id.unwrap_or_default(),
            name: room.name.clone(),
            slug: room.slug.clone(),
            status: room.status(),
            max_size: room.max_size,
            current_size: room.current_size,
            max_times_entered: room.max_times_entered,
            current_times_entered: room.current_times_entered,
            expire_at: room.expire_at,
            created_at: room.created_at,
            updated_at: room.updated_at,
            permission: room.permission,
            password_protected: room.password.is_some(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct VerifyRoomPasswordRequest {
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct VerifyRoomPasswordResponse {
    pub valid: bool,
}

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
    /// 新房间密码。字段缺失表示保持当前密码不变。
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub password: Option<String>,
    /// 显式移除当前房间密码，与 password 互斥。
    #[serde(default)]
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub remove_password: Option<bool>,
    /// 房间有效期（可选，单位：秒；必须属于部署配置允许的期限）
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub age_seconds: Option<i64>,
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
