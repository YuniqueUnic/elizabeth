//! 授权决策核心（纯函数：无 IO、无时钟、无随机，可表驱动测试）。

use std::fmt;

use crate::models::room::content::ContentType;
use crate::models::room::role::{Capability, Grant, Scope};

/// 进房身份：持有效 Room Token 的主体。
#[derive(Debug, Clone)]
pub struct Principal<'a> {
    pub jti: &'a str,
    pub room_id: i64,
    pub role: &'a str,
}

/// 被访问对象。请求时从 DB 组装。
#[derive(Debug, Clone)]
pub enum Resource<'a> {
    Room {
        room_id: i64,
    },
    Content {
        room_id: i64,
        content_type: ContentType,
        created_by_jti: Option<&'a str>,
    },
}

impl Resource<'_> {
    fn room_id(&self) -> i64 {
        match self {
            Resource::Room { room_id } => *room_id,
            Resource::Content { room_id, .. } => *room_id,
        }
    }

    fn created_by_jti(&self) -> Option<&str> {
        match self {
            Resource::Room { .. } => None,
            Resource::Content { created_by_jti, .. } => *created_by_jti,
        }
    }
}

/// 拒绝原因（fail-closed 的四种来源，随 403 透出便于排障）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyReason {
    RoleMissing,
    CapabilityMissing,
    ScopeOwnViolation,
    RoomMismatch,
}

impl fmt::Display for DenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            DenyReason::RoleMissing => "role no longer exists in room",
            DenyReason::CapabilityMissing => "role lacks the required capability",
            DenyReason::ScopeOwnViolation => "capability only covers own content",
            DenyReason::RoomMismatch => "resource belongs to another room",
        };
        f.write_str(text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny(DenyReason),
}

/// 决策核心，语义为默认拒绝：
///
/// Allow ⇔ 角色存在 ∧ resource.room_id == principal.room_id
///        ∧ 角色授予了请求能力 ∧ 作用域覆盖
///        （`Any` 恒覆盖；`Own` 仅当资源创建者 == principal.jti）。
///
/// `grants = None` 表示角色在房间内不存在（token 引用了已删除角色）。
/// `Own` 作用域只对 Content 资源有意义；Room 资源没有创建者，任何 `Own` 授权均拒绝。
pub fn authorize(
    grants: Option<&[Grant]>,
    principal: &Principal<'_>,
    capability: Capability,
    resource: &Resource<'_>,
) -> Decision {
    let Some(grants) = grants else {
        return Decision::Deny(DenyReason::RoleMissing);
    };
    if resource.room_id() != principal.room_id {
        return Decision::Deny(DenyReason::RoomMismatch);
    }
    let Some(grant) = grants.iter().find(|grant| grant.capability == capability) else {
        return Decision::Deny(DenyReason::CapabilityMissing);
    };
    let covered = match grant.scope {
        Scope::Any => true,
        Scope::Own => resource.created_by_jti() == Some(principal.jti),
    };
    if covered {
        Decision::Allow
    } else {
        Decision::Deny(DenyReason::ScopeOwnViolation)
    }
}
