//! 房间角色与权限（Authorization）模块
//!
//! 三层分离：Room Gate（密码/过期/次数/状态）≠ Authorization（本模块）≠ Resource Policy。
//! 业务代码只允许通过 [`Authz::require`] 鉴权；决策核心为纯函数（[`engine`]），
//! 角色矩阵来自每房间独立的 `room_roles` 表（[`roles`]，版本化缓存）。
//!
//! Capability/Scope/Grant 契约的单一真相在 `board-protocol::models::room::role`
//! （Rust 枚举 = serde 字符串 = ts-rs TS 类型 = DB 紧凑格式）。

mod engine;
mod guard;
mod roles;

pub use engine::{Decision, DenyReason, Principal, Resource, authorize};
pub use guard::{Authz, validate_grants};
pub use roles::{RoleTable, RoleTableCache, load_role_table};
