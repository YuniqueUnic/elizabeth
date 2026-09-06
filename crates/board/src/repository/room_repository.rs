use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sqlx::{Any, FromRow, Row};
use std::sync::Arc;

use crate::models::room::role::SYSTEM_ROLE_TEMPLATES;
use crate::models::room::role::grants_to_json;
use crate::models::room::row_utils::{format_naive_datetime, format_optional_naive_datetime};
use crate::{
    db::DbPool,
    models::{Room, RoomStatus},
};

const ROOM_SELECT_BASE: &str = r#"
    SELECT
        id,
        name,
        slug,
        password,
        status,
        max_size,
        current_size,
        max_times_entered,
        current_times_entered,
        CAST(expire_at AS TEXT) as expire_at,
        CAST(created_at AS TEXT) as created_at,
        CAST(updated_at AS TEXT) as updated_at,
        default_role_key,
        roles_version
    FROM rooms
"#;

#[async_trait]
pub trait IRoomRepository: Send + Sync {
    async fn exists(&self, name: &str) -> Result<bool>;
    async fn create(&self, room: &Room) -> Result<Room>;
    async fn create_if_absent(&self, room: &Room) -> Result<Option<Room>>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Room>>;
    async fn find_by_display_name(&self, name: &str) -> Result<Option<Room>>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Room>>;
    async fn update(&self, room: &Room) -> Result<Room>;
    async fn delete(&self, name: &str) -> Result<bool>;
    async fn list_expired(&self) -> Result<Vec<Room>>;
    async fn delete_expired_before(&self, before: NaiveDateTime) -> Result<u64>;
}

/// 通用房间仓库（兼容 Sqlite / Postgres）
pub struct RoomRepository {
    pool: Arc<DbPool>,
}

impl RoomRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_room_optional_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Room>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!("{ROOM_SELECT_BASE} WHERE id = $1");
        let room = sqlx::query_as::<_, Room>(&sql)
            .bind(id)
            .fetch_optional(executor)
            .await?;
        Ok(room)
    }

    async fn fetch_room_optional_by_slug<'e, E>(executor: E, slug: &str) -> Result<Option<Room>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!("{ROOM_SELECT_BASE} WHERE slug = $1");
        let room = sqlx::query_as::<_, Room>(&sql)
            .bind(slug)
            .fetch_optional(executor)
            .await?;
        Ok(room)
    }

    async fn fetch_room_optional_by_display_name<'e, E>(
        executor: E,
        name: &str,
    ) -> Result<Option<Room>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!("{ROOM_SELECT_BASE} WHERE name = $1");
        let room = sqlx::query_as::<_, Room>(&sql)
            .bind(name)
            .fetch_optional(executor)
            .await?;
        Ok(room)
    }

    async fn fetch_room_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<Room>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        Self::fetch_room_optional_by_id(executor, id)
            .await?
            .ok_or_else(|| anyhow!("room not found for id {}", id))
    }

    async fn fetch_expired_rooms<'e, E>(executor: E, before: NaiveDateTime) -> Result<Vec<Room>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let sql = format!(
            "{ROOM_SELECT_BASE} WHERE expire_at IS NOT NULL AND CAST(expire_at AS TEXT) < $1"
        );
        let rooms = sqlx::query_as::<_, Room>(&sql)
            .bind(format_naive_datetime(before))
            .fetch_all(executor)
            .await?;
        Ok(rooms)
    }

    pub async fn update_policy(&self, room: &Room) -> Result<Room> {
        let room_id = room
            .id
            .ok_or_else(|| anyhow!("room id is required for policy update"))?;
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query(
            r#"
            UPDATE rooms
            SET password = $1,
                max_size = $2,
                max_times_entered = $3,
                expire_at = $4,
                default_role_key = $5,
                updated_at = $6
            WHERE id = $7
            "#,
        )
        .bind(&room.password)
        .bind(room.max_size)
        .bind(room.max_times_entered)
        .bind(format_optional_naive_datetime(room.expire_at))
        .bind(&room.default_role_key)
        .bind(now)
        .bind(room_id)
        .execute(&mut *tx)
        .await?;
        let updated = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;
        tx.commit().await?;
        Ok(updated)
    }

    /// 角色矩阵写路径专用：bump 版本号使所有会话的 RoleTable 缓存立即失效。
    pub async fn bump_roles_version(&self, room_id: i64) -> Result<i64> {
        let version: i64 = sqlx::query_scalar(
            "UPDATE rooms SET roles_version = roles_version + 1 WHERE id = $1 RETURNING roles_version",
        )
        .bind(room_id)
        .fetch_one(&*self.pool)
        .await?;
        Ok(version)
    }

    /// 角色矩阵写路径的与 bump 版本号同事务的变体（供 RoleRepository 组合）。
    pub async fn bump_roles_version_in<'e, E>(executor: E, room_id: i64) -> Result<()>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query("UPDATE rooms SET roles_version = roles_version + 1 WHERE id = $1")
            .bind(room_id)
            .execute(executor)
            .await?;
        Ok(())
    }

    /// 新房间种子：同事务写入三个系统角色（§5.3 模板）。
    async fn seed_system_roles(tx: &mut sqlx::Transaction<'_, Any>, room_id: i64) -> Result<()> {
        for template in SYSTEM_ROLE_TEMPLATES.iter() {
            sqlx::query(
                r#"
                INSERT INTO room_roles (room_id, role_key, display_name, capabilities, is_system)
                VALUES ($1, $2, $3, $4, 1)
                "#,
            )
            .bind(room_id)
            .bind(template.key)
            .bind(template.display_name)
            .bind(grants_to_json(template.capabilities))
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    pub async fn release_display_name(&self, room_id: i64, new_name: &str) -> Result<Room> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query("UPDATE rooms SET name = $1, updated_at = $2 WHERE id = $3")
            .bind(new_name)
            .bind(now)
            .bind(room_id)
            .execute(&mut *tx)
            .await?;
        let updated = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;
        tx.commit().await?;
        Ok(updated)
    }
}

#[async_trait]
impl IRoomRepository for RoomRepository {
    async fn exists(&self, name: &str) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT CASE WHEN EXISTS(SELECT 1 FROM rooms WHERE slug = $1 OR name = $1) THEN 1 ELSE 0 END",
        )
        .bind(name)
        .fetch_one(&*self.pool)
        .await?;

        Ok(exists != 0)
    }

    async fn create(&self, room: &Room) -> Result<Room> {
        self.create_if_absent(room)
            .await?
            .ok_or_else(|| anyhow!("room already exists"))
    }

    async fn create_if_absent(&self, room: &Room) -> Result<Option<Room>> {
        if room.is_expired() {
            return Err(anyhow!("cannot create an already expired room"));
        }
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let now_str = format_naive_datetime(now);
        let expire_at = format_optional_naive_datetime(room.expire_at);

        let inserted_id: Option<i64> = sqlx::query_scalar(
            r#"
            INSERT INTO rooms (
                name, slug, password, status, max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, default_role_key, roles_version
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT DO NOTHING
            RETURNING id
            "#,
        )
        .bind(&room.name)
        .bind(&room.slug)
        .bind(&room.password)
        .bind(room.status)
        .bind(room.max_size)
        .bind(room.current_size)
        .bind(room.max_times_entered)
        .bind(room.current_times_entered)
        .bind(expire_at.clone())
        .bind(now_str.clone())
        .bind(now_str.clone())
        .bind(&room.default_role_key)
        .bind(room.roles_version)
        .fetch_optional(&mut *tx)
        .await?;

        let created_room = match inserted_id {
            Some(inserted_id) => {
                Self::seed_system_roles(&mut tx, inserted_id).await?;
                Some(Self::fetch_room_by_id_or_err(&mut *tx, inserted_id).await?)
            }
            None => None,
        };

        tx.commit().await?;
        Ok(created_room)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        Self::fetch_room_optional_by_slug(&*self.pool, name).await
    }

    async fn find_by_display_name(&self, name: &str) -> Result<Option<Room>> {
        Self::fetch_room_optional_by_display_name(&*self.pool, name).await
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Room>> {
        Self::fetch_room_optional_by_id(&*self.pool, id).await
    }

    async fn update(&self, room: &Room) -> Result<Room> {
        let room_id = room
            .id
            .ok_or_else(|| anyhow!("room id is required for update"))?;
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let now_str = format_naive_datetime(now);
        let expire_at = format_optional_naive_datetime(room.expire_at);

        sqlx::query(
            r#"
            UPDATE rooms SET
                password = $1, status = $2, max_size = $3, current_size = $4,
                max_times_entered = $5, current_times_entered = $6, expire_at = $7,
                updated_at = $8, slug = $9
            WHERE id = $10
            "#,
        )
        .bind(&room.password)
        .bind(room.status)
        .bind(room.max_size)
        .bind(room.current_size)
        .bind(room.max_times_entered)
        .bind(room.current_times_entered)
        .bind(expire_at)
        .bind(now_str)
        .bind(&room.slug)
        .bind(room_id)
        .execute(&mut *tx)
        .await?;

        let updated_room = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;

        tx.commit().await?;
        Ok(updated_room)
    }

    async fn delete(&self, name: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM rooms WHERE slug = $1")
            .bind(name)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list_expired(&self) -> Result<Vec<Room>> {
        let now = Utc::now().naive_utc();
        Self::fetch_expired_rooms(&*self.pool, now).await
    }

    async fn delete_expired_before(&self, before: NaiveDateTime) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM rooms WHERE expire_at IS NOT NULL AND CAST(expire_at AS TEXT) < $1",
        )
        .bind(format_naive_datetime(before))
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
