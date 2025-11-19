use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sqlx::{Any, FromRow, Row};
use std::sync::Arc;

use crate::models::room::row_utils::{format_naive_datetime, format_optional_naive_datetime};
use crate::{
    db::DbPool,
    models::{Room, RoomStatus, permission::RoomPermission},
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
        permission
    FROM rooms
"#;

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
        let sql = format!("{ROOM_SELECT_BASE} WHERE id = ?");
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
        let sql = format!("{ROOM_SELECT_BASE} WHERE slug = ?");
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
        let sql = format!("{ROOM_SELECT_BASE} WHERE name = ?");
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
            "{ROOM_SELECT_BASE} WHERE expire_at IS NOT NULL AND CAST(expire_at AS TEXT) < ?"
        );
        let rooms = sqlx::query_as::<_, Room>(&sql)
            .bind(format_naive_datetime(before))
            .fetch_all(executor)
            .await?;
        Ok(rooms)
    }

    async fn reset_if_expired(&self, room: Room) -> Result<Option<Room>> {
        if room.is_expired() {
            if let Some(id) = room.id {
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
impl IRoomRepository for RoomRepository {
    async fn exists(&self, name: &str) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rooms WHERE slug = ?)")
            .bind(name)
            .fetch_one(&*self.pool)
            .await?;

        Ok(exists != 0)
    }

    async fn create(&self, room: &Room) -> Result<Room> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let now_str = format_naive_datetime(now);
        let expire_at = format_optional_naive_datetime(room.expire_at);

        let inserted_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO rooms (
                name, slug, password, status, max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(i64::from(room.permission.bits()))
        .fetch_one(&mut *tx)
        .await?;

        let created_room = Self::fetch_room_by_id_or_err(&mut *tx, inserted_id).await?;

        tx.commit().await?;
        self.reset_if_expired(created_room)
            .await?
            .ok_or_else(|| anyhow!("created room expired immediately"))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        let room = Self::fetch_room_optional_by_slug(&*self.pool, name).await?;
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
        let now_str = format_naive_datetime(now);
        let expire_at = format_optional_naive_datetime(room.expire_at);

        sqlx::query(
            r#"
            UPDATE rooms SET
                password = ?, status = ?, max_size = ?, current_size = ?,
                max_times_entered = ?, current_times_entered = ?, expire_at = ?,
                updated_at = ?, permission = ?, slug = ?
            WHERE id = ?
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
        .bind(i64::from(room.permission.bits()))
        .bind(&room.slug)
        .bind(room_id)
        .execute(&mut *tx)
        .await?;

        let updated_room = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;

        tx.commit().await?;
        self.reset_if_expired(updated_room)
            .await?
            .ok_or_else(|| anyhow!("updated room expired unexpectedly"))
    }

    async fn delete(&self, name: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM rooms WHERE slug = ?")
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
            "DELETE FROM rooms WHERE expire_at IS NOT NULL AND CAST(expire_at AS TEXT) < ?",
        )
        .bind(format_naive_datetime(before))
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
