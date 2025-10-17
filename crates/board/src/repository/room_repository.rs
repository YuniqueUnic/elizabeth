use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::{Room, RoomStatus};

#[async_trait]
pub trait RoomRepository: Send + Sync {
    async fn exists(&self, name: &str) -> Result<bool>;
    async fn create(&self, room: &Room) -> Result<Room>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Room>>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Room>>;
    async fn update(&self, room: &Room) -> Result<Room>;
    async fn delete(&self, name: &str) -> Result<bool>;
    async fn list_expired(&self) -> Result<Vec<Room>>;
}

pub struct SqliteRoomRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoomRepository for SqliteRoomRepository {
    async fn exists(&self, name: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM rooms WHERE name = ?", name)
            .fetch_one(&*self.pool)
            .await?;

        Ok(count > 0)
    }

    async fn create(&self, room: &Room) -> Result<Room> {
        let now = Utc::now().naive_utc();

        sqlx::query!(
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
        .execute(&*self.pool)
        .await?;

        // 获取插入的记录
        let created_room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id, name, password, status as "status: RoomStatus", max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            FROM rooms
            WHERE name = ?
            "#,
            room.name
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(created_room)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        let room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id, name, password, status as "status: RoomStatus", max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            FROM rooms
            WHERE name = ?
            "#,
            name
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(room)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Room>> {
        let room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id, name, password, status as "status: RoomStatus", max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            FROM rooms
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(room)
    }

    async fn update(&self, room: &Room) -> Result<Room> {
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
            room.id
        )
        .execute(&*self.pool)
        .await?;

        // 查询更新后的记录
        let updated_room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id, name, password, status as "status: RoomStatus", max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            FROM rooms
            WHERE id = ?
            "#,
            room.id
        )
        .fetch_one(&*self.pool)
        .await?;

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

        let rooms = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id, name, password, status as "status: RoomStatus", max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            FROM rooms
            WHERE expire_at IS NOT NULL AND expire_at < ?
            "#,
            now
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(rooms)
    }
}
