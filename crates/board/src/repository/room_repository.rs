use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sqlx::{Executor, Sqlite};
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::{Room, RoomStatus, permission::RoomPermission};

#[async_trait]
pub trait IRoomRepository: Send + Sync {
    async fn exists(&self, name: &str) -> Result<bool>;
    async fn create(&self, room: &Room) -> Result<Room>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Room>>;
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
}

#[async_trait]
impl IRoomRepository for SqliteRoomRepository {
    async fn exists(&self, name: &str) -> Result<bool> {
        let exists: i64 =
            sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM rooms WHERE name = ?)", name)
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
                name, password, status, max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            room.name,
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
        Ok(created_room)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        Self::fetch_room_optional_by_name(&*self.pool, name).await
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

        sqlx::query!(
            r#"
            UPDATE rooms SET
                password = ?, status = ?, max_size = ?, current_size = ?,
            max_times_entered = ?, current_times_entered = ?, expire_at = ?,
            updated_at = ?, permission = ?
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
            room_id
        )
        .execute(&mut *tx)
        .await?;

        let updated_room = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;

        tx.commit().await?;
        Ok(updated_room)
    }

    async fn delete(&self, name: &str) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM rooms WHERE name = ?", name)
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
