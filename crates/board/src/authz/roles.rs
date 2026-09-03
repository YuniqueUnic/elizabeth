//! RoleTable 加载与版本化缓存（authz 模块中唯一带 IO 的部分）。
//!
//! 缓存键为 `(room_id, roles_version)`；一切角色写路径必须 bump
//! `rooms.roles_version`（repo 层同一事务内），保证改矩阵立即对全房会话生效。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use board_protocol::dto::RoleDefinition;

use crate::db::DbPool;
use crate::errors::{AppError, AppResult};
use crate::models::room::role::{Grant, RoomRole, parse_grants_json};

const ROLE_SELECT: &str = "SELECT room_id, role_key, display_name, capabilities, is_system FROM room_roles WHERE room_id = $1";

/// 一个房间的已解析角色矩阵。
#[derive(Debug, Default)]
pub struct RoleTable {
    entries: HashMap<String, RoleDefinition>,
}

impl RoleTable {
    fn build(rows: Vec<RoomRole>) -> Self {
        let mut entries = HashMap::with_capacity(rows.len());
        for row in rows {
            // fail-closed：非法能力条目不拒绝加载整表，仅把该角色置为空集并记 error。
            let capabilities = match parse_grants_json(&row.capabilities) {
                Ok(capabilities) => capabilities,
                Err(error) => {
                    log::error!(
                        "room {} role `{}` has invalid capabilities ({}); failing closed as empty",
                        row.room_id,
                        row.role_key,
                        error
                    );
                    Vec::new()
                }
            };
            entries.insert(
                row.role_key.clone(),
                RoleDefinition {
                    role_key: row.role_key,
                    display_name: row.display_name,
                    capabilities,
                    is_system: row.is_system,
                },
            );
        }
        Self { entries }
    }

    /// 角色的授权集；角色不存在时为 None（决策核心映射为 RoleMissing）。
    pub fn grants(&self, role_key: &str) -> Option<&[Grant]> {
        self.entries
            .get(role_key)
            .map(|role| role.capabilities.as_slice())
    }

    pub fn get(&self, role_key: &str) -> Option<&RoleDefinition> {
        self.entries.get(role_key)
    }

    pub fn contains_key(&self, role_key: &str) -> bool {
        self.entries.contains_key(role_key)
    }

    pub fn definitions(&self) -> Vec<RoleDefinition> {
        self.entries.values().cloned().collect()
    }
}

/// `(room_id, roles_version) -> RoleTable` 的进程内缓存。
#[derive(Default)]
pub struct RoleTableCache {
    inner: RwLock<HashMap<i64, (i64, Arc<RoleTable>)>>,
}

impl RoleTableCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn hit(&self, room_id: i64, roles_version: i64) -> Option<Arc<RoleTable>> {
        let guard = self.inner.read().ok()?;
        guard
            .get(&room_id)
            .filter(|(version, _)| *version == roles_version)
            .map(|(_, table)| table.clone())
    }

    fn store(&self, room_id: i64, roles_version: i64, table: Arc<RoleTable>) {
        if let Ok(mut guard) = self.inner.write() {
            guard.insert(room_id, (roles_version, table));
        }
    }
}

/// 读取房间的角色矩阵：命中缓存直取，否则查库回填。
pub async fn load_role_table(
    cache: &RoleTableCache,
    pool: &Arc<DbPool>,
    room_id: i64,
    roles_version: i64,
) -> AppResult<Arc<RoleTable>> {
    if let Some(table) = cache.hit(room_id, roles_version) {
        return Ok(table);
    }
    let rows = sqlx::query_as::<_, RoomRole>(ROLE_SELECT)
        .bind(room_id)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|error| AppError::internal(format!("Failed to load room roles: {error}")))?;
    let table = Arc::new(RoleTable::build(rows));
    cache.store(room_id, roles_version, table.clone());
    Ok(table)
}
