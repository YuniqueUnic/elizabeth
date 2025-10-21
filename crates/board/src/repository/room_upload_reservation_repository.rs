use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, Utc};
use sqlx::{Executor, Sqlite, Transaction};

use crate::db::DbPool;
use crate::models::{Room, RoomStatus, RoomUploadReservation, permission::RoomPermission};

#[async_trait]
pub trait IRoomUploadReservationRepository: Send + Sync {
    async fn reserve_upload(
        &self,
        room: &Room,
        token_jti: &str,
        file_manifest: &str,
        reserved_size: i64,
        ttl: Duration,
    ) -> Result<(RoomUploadReservation, Room)>;

    async fn consume_reservation(
        &self,
        reservation_id: i64,
        room_id: i64,
        token_jti: &str,
        actual_size: i64,
        manifest: &str,
    ) -> Result<Room>;

    async fn release_expired_by_room(&self, room_id: i64) -> Result<Option<Room>>;

    async fn release_if_pending(&self, reservation_id: i64) -> Result<Option<Room>>;

    async fn fetch_by_id(&self, reservation_id: i64) -> Result<Option<RoomUploadReservation>>;
}

pub struct SqliteRoomUploadReservationRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomUploadReservationRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn begin_transaction(&self) -> Result<Transaction<'_, Sqlite>> {
        Ok(self.pool.begin().await?)
    }

    async fn fetch_room_by_id<'e, E>(executor: E, room_id: i64) -> Result<Room>
    where
        E: sqlx::Executor<'e, Database = Sqlite>,
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
            room_id
        )
        .fetch_optional(executor)
        .await?;

        room.ok_or_else(|| anyhow!("room not found for id {}", room_id))
    }

    async fn cleanup_expired(
        tx: &mut Transaction<'_, Sqlite>,
        room_id: i64,
        now: NaiveDateTime,
    ) -> Result<i64> {
        let expired_rows = sqlx::query!(
            r#"
            SELECT id, reserved_size
            FROM room_upload_reservations
            WHERE room_id = ? AND consumed_at IS NULL AND expires_at <= ?
            "#,
            room_id,
            now
        )
        .fetch_all(tx.as_mut())
        .await?;

        if expired_rows.is_empty() {
            return Ok(0);
        }

        let mut total_released = 0_i64;
        for row in expired_rows {
            total_released += row.reserved_size;
            sqlx::query!("DELETE FROM room_upload_reservations WHERE id = ?", row.id)
                .execute(tx.as_mut())
                .await?;
        }

        if total_released > 0 {
            sqlx::query!(
                r#"
                UPDATE rooms
                SET current_size = CASE
                    WHEN current_size <= ? THEN 0
                    ELSE current_size - ?
                END
                WHERE id = ?
                "#,
                total_released,
                total_released,
                room_id
            )
            .execute(tx.as_mut())
            .await?;
        }

        Ok(total_released)
    }
}

#[async_trait]
impl IRoomUploadReservationRepository for SqliteRoomUploadReservationRepository {
    async fn reserve_upload(
        &self,
        room: &Room,
        token_jti: &str,
        file_manifest: &str,
        reserved_size: i64,
        ttl: Duration,
    ) -> Result<(RoomUploadReservation, Room)> {
        if reserved_size <= 0 {
            return Err(anyhow!("reserved size must be positive"));
        }

        let mut tx = self.begin_transaction().await?;
        let now = Utc::now().naive_utc();
        let room_id = room.id.ok_or_else(|| anyhow!("room id missing"))?;

        Self::cleanup_expired(&mut tx, room_id, now).await?;

        let current_room = Self::fetch_room_by_id(tx.as_mut(), room_id).await?;
        if current_room.current_size + reserved_size > current_room.max_size {
            return Err(anyhow!("room size limit exceeded"));
        }

        sqlx::query!(
            "UPDATE rooms SET current_size = current_size + ? WHERE id = ?",
            reserved_size,
            room_id
        )
        .execute(tx.as_mut())
        .await?;

        let expires_at = now + ttl;
        let insert = sqlx::query!(
            r#"
            INSERT INTO room_upload_reservations
                (room_id, token_jti, file_manifest, reserved_size, reserved_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            room_id,
            token_jti,
            file_manifest,
            reserved_size,
            now,
            expires_at
        )
        .execute(tx.as_mut())
        .await?;

        let inserted_id = insert.last_insert_rowid();
        let reservation = sqlx::query_as!(
            RoomUploadReservation,
            r#"
            SELECT
                id as "id: Option<i64>",
                room_id,
                token_jti,
                file_manifest,
                reserved_size,
                reserved_at,
                expires_at,
                consumed_at,
                created_at,
                updated_at
            FROM room_upload_reservations
            WHERE id = ?
            "#,
            inserted_id
        )
        .fetch_one(tx.as_mut())
        .await?;

        let updated_room = Self::fetch_room_by_id(tx.as_mut(), room_id).await?;

        tx.commit().await?;
        Ok((reservation, updated_room))
    }

    async fn consume_reservation(
        &self,
        reservation_id: i64,
        room_id: i64,
        token_jti: &str,
        actual_size: i64,
        manifest: &str,
    ) -> Result<Room> {
        if actual_size < 0 {
            return Err(anyhow!("actual size cannot be negative"));
        }

        let mut tx = self.begin_transaction().await?;
        let now = Utc::now().naive_utc();

        let reservation = sqlx::query_as!(
            RoomUploadReservation,
            r#"
            SELECT
                id as "id: Option<i64>",
                room_id,
                token_jti,
                file_manifest,
                reserved_size,
                reserved_at,
                expires_at,
                consumed_at,
                created_at,
                updated_at
            FROM room_upload_reservations
            WHERE id = ? AND room_id = ?
            "#,
            reservation_id,
            room_id
        )
        .fetch_optional(tx.as_mut())
        .await?
        .ok_or_else(|| anyhow!("reservation not found"))?;

        let res_id = reservation
            .id
            .ok_or_else(|| anyhow!("reservation persisted without id"))?;

        if reservation.token_jti != token_jti {
            return Err(anyhow!("reservation token mismatch"));
        }
        if reservation.consumed_at.is_some() {
            return Err(anyhow!("reservation already consumed"));
        }
        if reservation.expires_at < now {
            sqlx::query!("DELETE FROM room_upload_reservations WHERE id = ?", res_id)
                .execute(tx.as_mut())
                .await?;

            sqlx::query!(
                r#"
                UPDATE rooms
                SET current_size = CASE
                    WHEN current_size <= ? THEN 0
                    ELSE current_size - ?
                END
                WHERE id = ?
                "#,
                reservation.reserved_size,
                reservation.reserved_size,
                room_id
            )
            .execute(tx.as_mut())
            .await?;

            tx.commit().await?;
            return Err(anyhow!("reservation expired"));
        }

        if actual_size > reservation.reserved_size {
            return Err(anyhow!("actual upload size exceeds reservation"));
        }

        let released = reservation.reserved_size - actual_size;
        if released > 0 {
            sqlx::query!(
                r#"
                UPDATE rooms
                SET current_size = CASE
                    WHEN current_size <= ? THEN 0
                    ELSE current_size - ?
                END
                WHERE id = ?
                "#,
                released,
                released,
                room_id
            )
            .execute(tx.as_mut())
            .await?;
        }

        sqlx::query!(
            r#"
            UPDATE room_upload_reservations
            SET consumed_at = ?, file_manifest = ?, reserved_size = ?
            WHERE id = ?
            "#,
            now,
            manifest,
            actual_size,
            reservation_id
        )
        .execute(tx.as_mut())
        .await?;

        let updated_room = Self::fetch_room_by_id(tx.as_mut(), room_id).await?;
        tx.commit().await?;
        Ok(updated_room)
    }

    async fn release_expired_by_room(&self, room_id: i64) -> Result<Option<Room>> {
        let mut tx = self.begin_transaction().await?;
        let now = Utc::now().naive_utc();
        let released = Self::cleanup_expired(&mut tx, room_id, now).await?;
        if released > 0 {
            let room = Self::fetch_room_by_id(tx.as_mut(), room_id).await?;
            tx.commit().await?;
            Ok(Some(room))
        } else {
            tx.commit().await?;
            Ok(None)
        }
    }

    async fn release_if_pending(&self, reservation_id: i64) -> Result<Option<Room>> {
        let mut tx = self.begin_transaction().await?;
        let now = Utc::now().naive_utc();

        let reservation = sqlx::query!(
            r#"
            SELECT room_id, reserved_size, consumed_at, expires_at
            FROM room_upload_reservations
            WHERE id = ?
            "#,
            reservation_id
        )
        .fetch_optional(tx.as_mut())
        .await?;

        let Some(reservation) = reservation else {
            tx.commit().await?;
            return Ok(None);
        };

        if reservation.consumed_at.is_some() || reservation.expires_at > now {
            tx.commit().await?;
            return Ok(None);
        }

        sqlx::query!(
            "DELETE FROM room_upload_reservations WHERE id = ?",
            reservation_id
        )
        .execute(tx.as_mut())
        .await?;

        sqlx::query!(
            r#"
            UPDATE rooms
            SET current_size = CASE
                WHEN current_size <= ? THEN 0
                ELSE current_size - ?
            END
            WHERE id = ?
            "#,
            reservation.reserved_size,
            reservation.reserved_size,
            reservation.room_id
        )
        .execute(tx.as_mut())
        .await?;

        let room = Self::fetch_room_by_id(tx.as_mut(), reservation.room_id).await?;
        tx.commit().await?;
        Ok(Some(room))
    }

    async fn fetch_by_id(&self, reservation_id: i64) -> Result<Option<RoomUploadReservation>> {
        let record = sqlx::query_as!(
            RoomUploadReservation,
            r#"
            SELECT
                id as "id: Option<i64>",
                room_id,
                token_jti,
                file_manifest,
                reserved_size,
                reserved_at,
                expires_at,
                consumed_at,
                created_at,
                updated_at
            FROM room_upload_reservations
            WHERE id = ?
            "#,
            reservation_id
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(record)
    }
}
