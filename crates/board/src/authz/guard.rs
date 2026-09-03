//! Authz 上下文组装：把已通过身份与 Room Gate 校验的 claims 升级为可判定的授权上下文。
//!
//! Handler 纪律（唯一合法模式）：
//!
//! ```ignore
//! let authz = Authz::for_claims(&app_state, &room, &claims).await?;
//! authz.require(Capability::FileUpload, &Resource::Room { room_id: room.id.unwrap() })?;
//! ```
//!
//! 禁止：`claims.role == "admin"` 等字符串分支、直查 `room_roles` 表。

use board_protocol::models::room::role::{Capability, Grant, Scope};

use crate::errors::{AppError, AppResult};
use crate::models::Room;
use crate::models::room::role::GrantParseError;
use crate::services::RoomTokenClaims;
use crate::state::AppState;

use super::engine::{Decision, DenyReason, Principal, Resource, authorize};
use super::roles::load_role_table;

pub struct Authz<'a> {
    principal: Principal<'a>,
    grants: Option<Vec<Grant>>,
}

impl<'a> Authz<'a> {
    /// 组装授权上下文。调用前必须已完成 token 验签与 Room Gate（房间存在/未过期/开放）。
    pub async fn for_claims(
        app_state: &AppState,
        room: &Room,
        claims: &'a RoomTokenClaims,
    ) -> AppResult<Self> {
        let room_id = room
            .id
            .ok_or_else(|| AppError::internal("Room id missing"))?;
        if room_id != claims.room_id {
            return Err(AppError::authentication("Token room mismatch"));
        }
        let table = load_role_table(
            &app_state.roles_cache,
            &app_state.db_pool,
            room_id,
            room.roles_version,
        )
        .await?;
        let grants = table.grants(&claims.role).map(<[Grant]>::to_vec);
        Ok(Self {
            principal: Principal {
                jti: &claims.jti,
                room_id: claims.room_id,
                role: &claims.role,
            },
            grants,
        })
    }

    pub fn role(&self) -> &str {
        self.principal.role
    }

    pub fn jti(&self) -> &str {
        self.principal.jti
    }

    /// 已解析的能力集（角色不存在时为空切片）；供 token 响应回显等查询场景。
    pub fn capabilities(&self) -> &[Grant] {
        self.grants.as_deref().unwrap_or(&[])
    }

    /// 唯一合法的授权入口；拒绝统一映射为 403 PERMISSION_DENIED（附 DenyReason 便于排障）。
    pub fn require(&self, capability: Capability, resource: &Resource<'_>) -> AppResult<()> {
        match authorize(
            self.grants.as_deref(),
            &self.principal,
            capability,
            resource,
        ) {
            Decision::Allow => Ok(()),
            Decision::Deny(reason) => Err(Self::denied(capability, reason)),
        }
    }

    fn denied(capability: Capability, reason: DenyReason) -> AppError {
        AppError::permission_denied(format!(
            "Access denied: capability `{capability}` not granted ({reason})"
        ))
    }
}

/// 供角色 API 使用的 Grant 校验：key 可解析、非 ownable 能力不得带 Own 作用域。
/// 由 serde 先行解析成 `Vec<Grant>`，这里只补齐"Own 只允许出现在 ownable 能力上"的约束。
pub fn validate_grants(grants: &[Grant]) -> Result<(), GrantParseError> {
    for grant in grants {
        if grant.scope == Scope::Own && !grant.capability.is_ownable() {
            return Err(GrantParseError(format!(
                "capability `{}` does not accept own scope",
                grant.capability
            )));
        }
    }
    Ok(())
}
