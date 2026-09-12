//! `room_roles` 仓库：角色矩阵 CRUD，所有写路径与 `roles_version` bump 同事务提交。

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Any;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::role::RoomRole;
use crate::models::room::row_utils::format_naive_datetime;
use crate::repository::room_repository::RoomRepository;

#[async_trait]
pub trait IRoomRoleRepository: Send + Sync {
    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomRole>>;
    async fn find_by_key(&self, room_id: i64, role_key: &str) -> Result<Option<RoomRole>>;
    /// 创建自定义角色并 bump 版本号。
    async fn create(
        &self,
        room_id: i64,
        role_key: &str,
        display_name: &str,
        capabilities_json: &str,
    ) -> Result<RoomRole>;
    /// 整体替换角色定义并 bump 版本号。
    async fn update(
        &self,
        room_id: i64,
        role_key: &str,
        display_name: &str,
        capabilities_json: &str,
    ) -> Result<RoomRole>;
    /// 删除角色并 bump 版本号；返回受影响 token 数供调用方提示。
    async fn delete(&self, room_id: i64, role_key: &str) -> Result<bool>;
    /// 引用该角色的未撤销 token 数（删除前的 UI 提示用）。
    async fn count_active_tokens_with_role(&self, room_id: i64, role_key: &str) -> Result<i64>;
}

pub struct RoomRoleRepository {
    pool: Arc<DbPool>,
}

impl RoomRoleRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional<'e, E>(
        executor: E,
        room_id: i64,
        role_key: &str,
    ) -> Result<Option<RoomRole>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let role = sqlx::query_as::<_, RoomRole>(
            "SELECT room_id, role_key, display_name, capabilities, is_system
             FROM room_roles WHERE room_id = $1 AND role_key = $2",
        )
        .bind(room_id)
        .bind(role_key)
        .fetch_optional(executor)
        .await?;
        Ok(role)
    }
}

#[async_trait]
impl IRoomRoleRepository for RoomRoleRepository {
    async fn list_by_room(&self, room_id: i64) -> Result<Vec<RoomRole>> {
        let roles = sqlx::query_as::<_, RoomRole>(
            "SELECT room_id, role_key, display_name, capabilities, is_system
             FROM room_roles WHERE room_id = $1
             ORDER BY is_system DESC, role_key",
        )
        .bind(room_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(roles)
    }

    async fn find_by_key(&self, room_id: i64, role_key: &str) -> Result<Option<RoomRole>> {
        Self::fetch_optional(&*self.pool, room_id, role_key).await
    }

    async fn create(
        &self,
        room_id: i64,
        role_key: &str,
        display_name: &str,
        capabilities_json: &str,
    ) -> Result<RoomRole> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query(
            r#"
            INSERT INTO room_roles (room_id, role_key, display_name, capabilities, is_system, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 0, $5, $6)
            "#,
        )
        .bind(room_id)
        .bind(role_key)
        .bind(display_name)
        .bind(capabilities_json)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        RoomRepository::bump_roles_version_in(&mut *tx, room_id).await?;
        let created = Self::fetch_optional(&mut *tx, room_id, role_key)
            .await?
            .ok_or_else(|| anyhow!("created role not found"))?;
        tx.commit().await?;
        Ok(created)
    }

    async fn update(
        &self,
        room_id: i64,
        role_key: &str,
        display_name: &str,
        capabilities_json: &str,
    ) -> Result<RoomRole> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        let result = sqlx::query(
            r#"
            UPDATE room_roles
            SET display_name = $3, capabilities = $4, updated_at = $5
            WHERE room_id = $1 AND role_key = $2
            "#,
        )
        .bind(room_id)
        .bind(role_key)
        .bind(display_name)
        .bind(capabilities_json)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(anyhow!("role not found"));
        }
        RoomRepository::bump_roles_version_in(&mut *tx, room_id).await?;
        let updated = Self::fetch_optional(&mut *tx, room_id, role_key)
            .await?
            .ok_or_else(|| anyhow!("updated role not found"))?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn delete(&self, room_id: i64, role_key: &str) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query("DELETE FROM room_roles WHERE room_id = $1 AND role_key = $2")
            .bind(room_id)
            .bind(role_key)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        RoomRepository::bump_roles_version_in(&mut *tx, room_id).await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn count_active_tokens_with_role(&self, room_id: i64, role_key: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM room_tokens
             WHERE room_id = $1 AND role_key = $2 AND revoked_at IS NULL",
        )
        .bind(room_id)
        .bind(role_key)
        .fetch_one(&*self.pool)
        .await?;
        Ok(count)
    }
}
