use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sqlx::{Executor, Sqlite};
use std::sync::Arc;

use crate::{
    db::DbPool,
    models::{Room, RoomStatus, permission::RoomPermission},
};

#[async_trait]
pub trait IRoomRepository: Send + Sync {
    async fn exists(&self, name: &str) -> Result<bool>;
    async fn create(&self, room: &Room) -> Result<Room>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Room>>;
    async fn find_by_display_name(&self, name: &str) -> Result<Option<Room>>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Room>>;
    async fn update(&self, room: &Room) -> Result<Room>;
    async fn delete(&self, name: &str) -> Result<bool>;
    async fn list_expired(&self) -> Result<Vec<Room>>;
    async fn delete_expired_before(&self, before: NaiveDateTime) -> Result<u64>;
}

pub struct SqliteRoomRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_room_optional_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Room>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id,
                name,
                slug,
                password,
                status as "status: RoomStatus",
                max_size,
                current_size,
                max_times_entered,
                current_times_entered,
                expire_at,
                created_at,
                updated_at,
                permission as "permission: RoomPermission"
            FROM rooms
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(room)
    }

    async fn fetch_room_optional_by_name<'e, E>(executor: E, name: &str) -> Result<Option<Room>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id,
                name,
                slug,
                password,
                status as "status: RoomStatus",
                max_size,
                current_size,
                max_times_entered,
                current_times_entered,
                expire_at,
                created_at,
                updated_at,
                permission as "permission: RoomPermission"
            FROM rooms
            WHERE slug = ?
            "#,
            name
        )
        .fetch_optional(executor)
        .await?;

        Ok(room)
    }

    async fn fetch_room_optional_by_display_name<'e, E>(
        executor: E,
        name: &str,
    ) -> Result<Option<Room>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id,
                name,
                slug,
                password,
                status as "status: RoomStatus",
                max_size,
                current_size,
                max_times_entered,
                current_times_entered,
                expire_at,
                created_at,
                updated_at,
                permission as "permission: RoomPermission"
            FROM rooms
            WHERE name = ?
            "#,
            name
        )
        .fetch_optional(executor)
        .await?;

        Ok(room)
    }

    async fn fetch_room_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<Room>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        Self::fetch_room_optional_by_id(executor, id)
            .await?
            .ok_or_else(|| anyhow!("room not found for id {}", id))
    }

    async fn fetch_expired_rooms<'e, E>(executor: E, before: NaiveDateTime) -> Result<Vec<Room>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let rooms = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id,
                name,
                slug,
                password,
                status as "status: RoomStatus",
                max_size,
                current_size,
                max_times_entered,
                current_times_entered,
                expire_at,
                created_at,
                updated_at,
                permission as "permission: RoomPermission"
            FROM rooms
            WHERE expire_at IS NOT NULL AND expire_at < ?
            "#,
            before
        )
        .fetch_all(executor)
        .await?;

        Ok(rooms)
    }

    async fn reset_if_expired(&self, room: Room) -> Result<Option<Room>> {
        if room.is_expired() {
            if let Some(id) = room.id {
                // 清空房间内容后删除房间记录
                sqlx::query("DELETE FROM room_contents WHERE room_id = ?")
                    .bind(id)
                    .execute(&*self.pool)
                    .await?;

                sqlx::query("DELETE FROM rooms WHERE id = ?")
                    .bind(id)
                    .execute(&*self.pool)
                    .await?;

                logrs::info!("Purged expired room {}", room.slug);

                return Ok(None);
            }
            Ok(None)
        } else {
            Ok(Some(room))
        }
    }
}

#[async_trait]
impl IRoomRepository for SqliteRoomRepository {
    async fn exists(&self, name: &str) -> Result<bool> {
        let exists: i64 =
            sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM rooms WHERE slug = ?)", name)
                .fetch_one(&*self.pool)
                .await?;

        Ok(exists != 0)
    }

    async fn create(&self, room: &Room) -> Result<Room> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();

        let insert_result = sqlx::query!(
            r#"
            INSERT INTO rooms (
                name, slug, password, status, max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            room.name,
            room.slug,
            room.password,
            room.status,
            room.max_size,
            room.current_size,
            room.max_times_entered,
            room.current_times_entered,
            room.expire_at,
            now,
            now,
            room.permission,
        )
        .execute(&mut *tx)
        .await?;

        let room_id = insert_result.last_insert_rowid();
        let created_room = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;

        tx.commit().await?;
        self.reset_if_expired(created_room)
            .await?
            .ok_or_else(|| anyhow!("created room expired immediately"))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        let room = Self::fetch_room_optional_by_name(&*self.pool, name).await?;
        if let Some(room) = room {
            self.reset_if_expired(room).await
        } else {
            Ok(None)
        }
    }

    async fn find_by_display_name(&self, name: &str) -> Result<Option<Room>> {
        let room = Self::fetch_room_optional_by_display_name(&*self.pool, name).await?;
        if let Some(room) = room {
            self.reset_if_expired(room).await
        } else {
            Ok(None)
        }
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Room>> {
        let room = Self::fetch_room_optional_by_id(&*self.pool, id).await?;
        if let Some(room) = room {
            self.reset_if_expired(room).await
        } else {
            Ok(None)
        }
    }

    async fn update(&self, room: &Room) -> Result<Room> {
        let room_id = room
            .id
            .ok_or_else(|| anyhow!("room id is required for update"))?;
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();

        sqlx::query!(
            r#"
            UPDATE rooms SET
                password = ?, status = ?, max_size = ?, current_size = ?,
            max_times_entered = ?, current_times_entered = ?, expire_at = ?,
            updated_at = ?, permission = ?, slug = ?
        WHERE id = ?
            "#,
            room.password,
            room.status,
            room.max_size,
            room.current_size,
            room.max_times_entered,
            room.current_times_entered,
            room.expire_at,
            now,
            room.permission,
            room.slug,
            room_id
        )
        .execute(&mut *tx)
        .await?;

        let updated_room = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;

        tx.commit().await?;
        self.reset_if_expired(updated_room)
            .await?
            .ok_or_else(|| anyhow!("updated room expired unexpectedly"))
    }

    async fn delete(&self, name: &str) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM rooms WHERE slug = ?", name)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list_expired(&self) -> Result<Vec<Room>> {
        let now = Utc::now().naive_utc();

        Self::fetch_expired_rooms(&*self.pool, now).await
    }

    async fn delete_expired_before(&self, before: NaiveDateTime) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM rooms WHERE expire_at IS NOT NULL AND expire_at < ?",
            before
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
