use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Any, AnyPool, QueryBuilder, Row};
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::upload_reservation::{RoomUploadReservation, UploadStatus};

#[async_trait]
pub trait IRoomUploadReservationRepository: Send + Sync {
    async fn create(&self, reservation: &RoomUploadReservation) -> Result<RoomUploadReservation>;
    async fn find_by_reservation_id(&self, reservation_id: &str) -> Result<Option<RoomUploadReservation>>;
    async fn mark_uploaded(&self, reservation_id: &str) -> Result<RoomUploadReservation>;
    async fn delete(&self, reservation_id: &str) -> Result<bool>;
    async fn purge_expired(&self) -> Result<u64>;
}

pub struct SqliteRoomUploadReservationRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomUploadReservationRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional<'e, E>(executor: E, reservation_id: &str) -> Result<Option<RoomUploadReservation>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomUploadReservation>(
            r#"
            SELECT id, room_id, reservation_id, file_name, expected_size, mime_type,
                   expire_at, status as "status: _", created_at
            FROM room_upload_reservations
            WHERE reservation_id = ?
            "#,
        )
        .bind(reservation_id)
        .fetch_optional(executor)
        .await
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<RoomUploadReservation>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, RoomUploadReservation>(
            r#"
            SELECT id, room_id, reservation_id, file_name, expected_size, mime_type,
                   expire_at, status as "status: _", created_at
            FROM room_upload_reservations
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| anyhow!("upload reservation not found"))
    }
}

#[async_trait]
impl IRoomUploadReservationRepository for SqliteRoomUploadReservationRepository {
    async fn create(&self, reservation: &RoomUploadReservation) -> Result<RoomUploadReservation> {
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_upload_reservations (
                room_id, reservation_id, file_name, expected_size, mime_type,
                expire_at, status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(reservation.room_id)
        .bind(&reservation.reservation_id)
        .bind(&reservation.file_name)
        .bind(reservation.expected_size)
        .bind(&reservation.mime_type)
        .bind(reservation.expire_at)
        .bind(reservation.status)
        .bind(reservation.created_at)
        .fetch_one(&mut *tx)
        .await?;

        let created = Self::fetch_by_id_or_err(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn find_by_reservation_id(&self, reservation_id: &str) -> Result<Option<RoomUploadReservation>> {
        Self::fetch_optional(&*self.pool, reservation_id).await
    }

    async fn mark_uploaded(&self, reservation_id: &str) -> Result<RoomUploadReservation> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            UPDATE room_upload_reservations
            SET status = ?, expire_at = ?, created_at = created_at
            WHERE reservation_id = ?
            "#,
        )
        .bind(UploadStatus::Completed)
        .bind(Utc::now().naive_utc())
        .bind(reservation_id)
        .execute(&mut *tx)
        .await?;

        let updated = SqliteRoomUploadReservationRepository::fetch_optional(&mut *tx, reservation_id)
            .await?
            .ok_or_else(|| anyhow!("upload reservation not found"))?;

        tx.commit().await?;
        Ok(updated)
    }

    async fn delete(&self, reservation_id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM room_upload_reservations WHERE reservation_id = ?")
            .bind(reservation_id)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn purge_expired(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM room_upload_reservations WHERE expire_at <= ?")
            .bind(Utc::now().naive_utc())
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
