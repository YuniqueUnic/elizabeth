//! 房间角色 API DTO（角色矩阵 CRUD 契约）

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::room::role::Grant;

/// 房间内一个角色的对外视图。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RoleDefinition {
    pub role_key: String,
    pub display_name: String,
    pub capabilities: Vec<Grant>,
    pub is_system: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct CreateRoleRequest {
    /// 自定义角色 key（slug 形式）；系统角色 key 保留字，不可占用
    pub role_key: String,
    pub display_name: String,
    pub capabilities: Vec<Grant>,
}

/// 整体替换角色定义（PUT 语义；系统角色仅可改能力集与显示名，key 与归属不可变）。
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UpdateRoleRequest {
    pub display_name: String,
    pub capabilities: Vec<Grant>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct DeleteRoleResponse {
    pub deleted: bool,
    /// 删除时仍绑定该角色的未撤销 token 数（这些会话将立即失去该角色授权）
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub affected_tokens: i64,
}
