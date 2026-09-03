//! 房间角色与能力契约（授权模型的单一真相）
//!
//! `Capability`/`Scope`/`Grant` 在 Rust 枚举、serde 字符串、ts-rs TS 类型与
//! `room_roles.capabilities` JSON 之间保持同一套字符串，任何一端不得私造。
//! `Capability` 的 serde rename 即线上字符串，也是数据库紧凑格式的能力段。

use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow, postgres::PgRow, sqlite::SqliteRow};
use utoipa::ToSchema;

/// own/any 作用域：`Any` 覆盖任何人的资源；`Own` 仅覆盖自己是创建者的资源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, Default)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub enum Scope {
    #[default]
    Any,
    Own,
}

/// 一个可鉴权的原子动作。
///
/// serde rename 字符串（如 `msg.edit`）同时是 ts-rs TS union、
/// `room_roles.capabilities` 紧凑格式与前端 i18n key 的能力段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub enum Capability {
    #[serde(rename = "room.share")]
    RoomShare,
    #[serde(rename = "room.settings.update")]
    RoomSettingsUpdate,
    #[serde(rename = "room.roles.manage")]
    RoomRolesManage,
    #[serde(rename = "room.delete")]
    RoomDelete,
    #[serde(rename = "msg.read")]
    MsgRead,
    #[serde(rename = "msg.send")]
    MsgSend,
    #[serde(rename = "msg.copy")]
    MsgCopy,
    #[serde(rename = "msg.edit")]
    MsgEdit,
    #[serde(rename = "msg.delete")]
    MsgDelete,
    #[serde(rename = "file.list")]
    FileList,
    #[serde(rename = "file.preview")]
    FilePreview,
    #[serde(rename = "file.download")]
    FileDownload,
    #[serde(rename = "file.upload")]
    FileUpload,
    #[serde(rename = "file.delete")]
    FileDelete,
    #[serde(rename = "file.policy.manage")]
    FilePolicyManage,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = serde_json::to_value(self)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default();
        f.write_str(&key)
    }
}

impl Capability {
    /// 全量能力表（与 utoipa/ts-rs 契约一致）。
    pub const ALL: [Capability; 15] = [
        Capability::RoomShare,
        Capability::RoomSettingsUpdate,
        Capability::RoomRolesManage,
        Capability::RoomDelete,
        Capability::MsgRead,
        Capability::MsgSend,
        Capability::MsgCopy,
        Capability::MsgEdit,
        Capability::MsgDelete,
        Capability::FileList,
        Capability::FilePreview,
        Capability::FileDownload,
        Capability::FileUpload,
        Capability::FileDelete,
        Capability::FilePolicyManage,
    ];

    /// 可配置 own/any 作用域的能力；其余能力只能以 Any 授予。
    pub fn is_ownable(self) -> bool {
        matches!(
            self,
            Capability::MsgEdit | Capability::MsgDelete | Capability::FileDelete
        )
    }
}

/// 最小授权单元：能力 + 作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct Grant {
    pub capability: Capability,
    pub scope: Scope,
}

impl Grant {
    pub const fn any(capability: Capability) -> Self {
        Self {
            capability,
            scope: Scope::Any,
        }
    }

    pub const fn own(capability: Capability) -> Self {
        Self {
            capability,
            scope: Scope::Own,
        }
    }

    /// 紧凑格式：`msg.edit`（Any）或 `msg.delete:own`。
    pub fn to_compact(self) -> String {
        let key = serde_json::to_value(self.capability)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default();
        match self.scope {
            Scope::Any => key,
            Scope::Own => format!("{key}:own"),
        }
    }
}

/// Grant 解析失败（角色矩阵中的非法条目，加载时按 fail-closed 处理）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantParseError(pub String);

impl fmt::Display for GrantParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid capability grant: {}", self.0)
    }
}

impl std::error::Error for GrantParseError {}

fn key_to_type<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(key.to_string())).ok()
}

/// 解析单条紧凑 Grant。
///
/// - 未知能力字符串 → Err（fail-closed）
/// - 不可配 Scope 的能力带 `:scope` 后缀 → Err（fail-closed）
/// - 可配 Scope 的能力缺后缀 → 视为 Any
pub fn parse_grant(raw: &str) -> Result<Grant, GrantParseError> {
    let (cap_key, scope_key) = match raw.split_once(':') {
        Some((capability, scope)) => (capability, Some(scope)),
        None => (raw, None),
    };
    let capability: Capability = key_to_type(cap_key)
        .ok_or_else(|| GrantParseError(format!("unknown capability `{cap_key}`")))?;
    match scope_key {
        Some(scope_key) => {
            if !capability.is_ownable() {
                return Err(GrantParseError(format!(
                    "capability `{cap_key}` does not accept a scope"
                )));
            }
            let scope: Scope = key_to_type(scope_key).ok_or_else(|| {
                GrantParseError(format!("unknown scope `{scope_key}` for `{cap_key}`"))
            })?;
            Ok(Grant { capability, scope })
        }
        None => Ok(Grant::any(capability)),
    }
}

/// 解析 `room_roles.capabilities`（紧凑字符串的 JSON 数组）。
///
/// 任一条目非法即整体 Err；调用方（RoleTable 加载器）按 fail-closed 将该角色视为空集。
pub fn parse_grants_json(json: &str) -> Result<Vec<Grant>, GrantParseError> {
    let entries: Vec<String> = serde_json::from_str(json).map_err(|error| {
        GrantParseError(format!("capabilities is not a JSON string array: {error}"))
    })?;
    entries.iter().map(|raw| parse_grant(raw)).collect()
}

/// 序列化为 `room_roles.capabilities` 紧凑 JSON。
pub fn grants_to_json(grants: &[Grant]) -> String {
    let entries: Vec<String> = grants.iter().map(|grant| grant.to_compact()).collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}

pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_EDITOR: &str = "editor";
pub const ROLE_READER: &str = "reader";
pub const MAX_EDITOR_TOKENS: i64 = 10;

/// 新房间默认加入角色；房间创建者通过独立 admin identity code 管理矩阵。
pub const DEFAULT_ROLE_KEY: &str = ROLE_READER;

pub struct SystemRoleTemplate {
    pub key: &'static str,
    pub display_name: &'static str,
    pub capabilities: &'static [Grant],
}

/// 系统角色模板：仅作为新建房间的种子；落库后以房间数据为准。
pub const SYSTEM_ROLE_TEMPLATES: [SystemRoleTemplate; 3] = [
    SystemRoleTemplate {
        key: ROLE_ADMIN,
        display_name: "Admin",
        capabilities: &[
            Grant::any(Capability::RoomShare),
            Grant::any(Capability::RoomSettingsUpdate),
            Grant::any(Capability::RoomRolesManage),
            Grant::any(Capability::RoomDelete),
            Grant::any(Capability::MsgRead),
            Grant::any(Capability::MsgSend),
            Grant::any(Capability::MsgCopy),
            Grant::any(Capability::MsgEdit),
            Grant::any(Capability::MsgDelete),
            Grant::any(Capability::FileList),
            Grant::any(Capability::FilePreview),
            Grant::any(Capability::FileDownload),
            Grant::any(Capability::FileUpload),
            Grant::any(Capability::FileDelete),
            Grant::any(Capability::FilePolicyManage),
        ],
    },
    SystemRoleTemplate {
        key: ROLE_EDITOR,
        display_name: "Editor",
        capabilities: &[
            Grant::any(Capability::MsgRead),
            Grant::any(Capability::MsgSend),
            Grant::any(Capability::MsgCopy),
            Grant::any(Capability::MsgEdit),
            Grant::own(Capability::MsgDelete),
            Grant::any(Capability::FileList),
            Grant::any(Capability::FilePreview),
            Grant::any(Capability::FileDownload),
            Grant::any(Capability::FileUpload),
            Grant::own(Capability::FileDelete),
        ],
    },
    SystemRoleTemplate {
        key: ROLE_READER,
        display_name: "Reader",
        capabilities: &[
            Grant::any(Capability::MsgRead),
            Grant::any(Capability::MsgCopy),
            Grant::any(Capability::FileList),
            Grant::any(Capability::FilePreview),
            Grant::any(Capability::FileDownload),
        ],
    },
];

/// 判断是否为系统角色 key。
pub fn is_system_role_key(key: &str) -> bool {
    SYSTEM_ROLE_TEMPLATES
        .iter()
        .any(|template| template.key == key)
}

/// 数据库 `room_roles` 行模型（capabilities 保持原始 JSON，解析交给授权层）。
/// 内部行模型，不进 OpenAPI / ts-rs 契约；对外走 `dto::RoleDefinition`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRole {
    pub room_id: i64,
    pub role_key: String,
    pub display_name: String,
    /// 紧凑 Grant 字符串的 JSON 数组（`["msg.edit:any", ...]`）
    pub capabilities: String,
    pub is_system: bool,
}

fn build_room_role_sqlite(row: &SqliteRow) -> Result<RoomRole, sqlx::Error> {
    Ok(RoomRole {
        room_id: row.try_get("room_id")?,
        role_key: row.try_get("role_key")?,
        display_name: row.try_get("display_name")?,
        capabilities: row.try_get("capabilities")?,
        is_system: row.try_get::<i64, _>("is_system")? != 0,
    })
}

fn build_room_role_pg(row: &PgRow) -> Result<RoomRole, sqlx::Error> {
    Ok(RoomRole {
        room_id: row.try_get("room_id")?,
        role_key: row.try_get("role_key")?,
        display_name: row.try_get("display_name")?,
        capabilities: row.try_get("capabilities")?,
        is_system: row.try_get::<bool, _>("is_system")?,
    })
}

fn build_room_role_any(row: &AnyRow) -> Result<RoomRole, sqlx::Error> {
    Ok(RoomRole {
        room_id: row.try_get("room_id")?,
        role_key: row.try_get("role_key")?,
        display_name: row.try_get("display_name")?,
        capabilities: row.try_get("capabilities")?,
        is_system: row.try_get::<i64, _>("is_system")? != 0,
    })
}

impl<'r> FromRow<'r, SqliteRow> for RoomRole {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_room_role_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for RoomRole {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_room_role_pg(row)
    }
}

impl<'r> FromRow<'r, AnyRow> for RoomRole {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_room_role_any(row)
    }
}
