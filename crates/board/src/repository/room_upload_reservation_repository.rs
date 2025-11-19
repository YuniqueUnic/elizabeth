use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use sqlx::Any;
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::room::Room;
use crate::models::room::row_utils::{format_naive_datetime, format_optional_naive_datetime};
use crate::models::room::upload_reservation::{RoomUploadReservation, UploadStatus};

const SELECT_BASE: &str = r#"
    SELECT
        id,
        room_id,
        token_jti,
        file_manifest,
        reserved_size,
        CAST(reserved_at AS TEXT) as reserved_at,
        CAST(expires_at AS TEXT) as expires_at,
        CAST(consumed_at AS TEXT) as consumed_at,
        CAST(created_at AS TEXT) as created_at,
        CAST(updated_at AS TEXT) as updated_at,
        CAST(chunked_upload AS INTEGER) as chunked_upload,
        total_chunks,
        uploaded_chunks,
        file_hash,
        chunk_size,
        upload_status
    FROM room_upload_reservations
"#;

#[async_trait]
pub trait IRoomUploadReservationRepository: Send + Sync {
    async fn create(&self, reservation: &RoomUploadReservation) -> Result<RoomUploadReservation>;
    async fn find_by_reservation_id(
        &self,
        reservation_id: i64,
    ) -> Result<Option<RoomUploadReservation>>;
    async fn find_by_token(&self, token_jti: &str) -> Result<Option<RoomUploadReservation>>;
    async fn reserve_upload(
        &self,
        room: &Room,
        token_jti: &str,
        file_manifest: &str,
        reserved_size: i64,
        ttl: Duration,
    ) -> Result<(RoomUploadReservation, Room)>;
    async fn fetch_by_id(&self, reservation_id: i64) -> Result<Option<RoomUploadReservation>>;
    async fn release_if_pending(&self, reservation_id: i64) -> Result<()>;
    async fn consume_reservation(
        &self,
        reservation_id: i64,
        room_id: i64,
        token_jti: &str,
        actual_size: i64,
        manifest: &str,
    ) -> Result<Room>;
    async fn update_uploaded_chunks(
        &self,
        reservation_id: i64,
        uploaded_chunks: i64,
    ) -> Result<RoomUploadReservation>;
    async fn update_upload_status(
        &self,
        reservation_id: i64,
        status: UploadStatus,
    ) -> Result<RoomUploadReservation>;
    async fn consume_upload(&self, reservation_id: i64) -> Result<RoomUploadReservation>;
    async fn mark_uploaded(&self, reservation_id: i64) -> Result<RoomUploadReservation>;
    async fn delete(&self, reservation_id: i64) -> Result<bool>;
    async fn purge_expired(&self) -> Result<u64>;
}

pub struct RoomUploadReservationRepository {
    pool: Arc<DbPool>,
}

impl RoomUploadReservationRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional<'e, E>(
        executor: E,
        reservation_id: i64,
    ) -> Result<Option<RoomUploadReservation>>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        let row = sqlx::query_as::<_, RoomUploadReservation>(&format!("{SELECT_BASE} WHERE id = ?"))
            .bind(reservation_id)
            .fetch_optional(executor)
            .await?;
        Ok(row)
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<RoomUploadReservation>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        Self::fetch_optional(executor, id)
            .await?
            .ok_or_else(|| anyhow!("upload reservation not found"))
    }

    async fn fetch_room<'e, E>(executor: E, room_id: i64) -> Result<Room>
    where
        E: sqlx::Executor<'e, Database = Any>,
    {
        sqlx::query_as::<_, Room>(
            r#"
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
            WHERE id = ?
            "#,
        )
        .bind(room_id)
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| anyhow!("room not found for reservation"))
    }
}

#[async_trait]
impl IRoomUploadReservationRepository for RoomUploadReservationRepository {
    async fn create(&self, reservation: &RoomUploadReservation) -> Result<RoomUploadReservation> {
        let mut tx = self.pool.begin().await?;
        let reserved_at = format_naive_datetime(reservation.reserved_at);
        let expires_at = format_naive_datetime(reservation.expires_at);
        let consumed_at = format_optional_naive_datetime(reservation.consumed_at);
        let created_at = format_naive_datetime(reservation.created_at);
        let updated_at = format_naive_datetime(reservation.updated_at);
        let upload_status = reservation
            .upload_status
            .clone()
            .unwrap_or(UploadStatus::Pending);
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_upload_reservations (
                room_id,
                token_jti,
                file_manifest,
                reserved_size,
                reserved_at,
                expires_at,
                consumed_at,
                created_at,
                updated_at,
                chunked_upload,
                total_chunks,
                uploaded_chunks,
                file_hash,
                chunk_size,
                upload_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(reservation.room_id)
        .bind(&reservation.token_jti)
        .bind(&reservation.file_manifest)
        .bind(reservation.reserved_size)
        .bind(reserved_at)
        .bind(expires_at)
        .bind(consumed_at)
        .bind(created_at)
        .bind(updated_at)
        .bind(reservation.chunked_upload.unwrap_or(false))
        .bind(reservation.total_chunks)
        .bind(reservation.uploaded_chunks)
        .bind(&reservation.file_hash)
        .bind(reservation.chunk_size)
        .bind(upload_status)
        .fetch_one(&mut *tx)
        .await?;

        let created = Self::fetch_by_id_or_err(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn find_by_reservation_id(
        &self,
        reservation_id: i64,
    ) -> Result<Option<RoomUploadReservation>> {
        Self::fetch_optional(&*self.pool, reservation_id).await
    }

    async fn find_by_token(&self, token_jti: &str) -> Result<Option<RoomUploadReservation>> {
        let row =
            sqlx::query_as::<_, RoomUploadReservation>(&format!("{SELECT_BASE} WHERE token_jti = ?"))
                .bind(token_jti)
                .fetch_optional(&*self.pool)
                .await?;
        Ok(row)
    }

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
        let room_id = room
            .id
            .ok_or_else(|| anyhow!("room id required for reservation"))?;

        let mut tx = self.pool.begin().await?;
        let mut latest_room = Self::fetch_room(&mut *tx, room_id).await?;
        if !latest_room.can_add_content(reserved_size) {
            return Err(anyhow!("room size limit exceeded"));
        }

        latest_room.current_size += reserved_size;
        let now = Utc::now().naive_utc();
        let expires_at = now + ttl;
        let now_str = format_naive_datetime(now);
        let expires_at_str = format_naive_datetime(expires_at);

        sqlx::query(
            r#"
            UPDATE rooms
            SET current_size = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(latest_room.current_size)
        .bind(now_str.clone())
        .bind(room_id)
        .execute(&mut *tx)
        .await?;

        let reservation_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO room_upload_reservations (
                room_id,
                token_jti,
                file_manifest,
                reserved_size,
                reserved_at,
                expires_at,
                consumed_at,
                created_at,
                updated_at,
                chunked_upload,
                total_chunks,
                uploaded_chunks,
                file_hash,
                chunk_size,
                upload_status
            )
            VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?, FALSE, NULL, NULL, NULL, NULL, ?)
            RETURNING id
            "#,
        )
        .bind(room_id)
        .bind(token_jti)
        .bind(file_manifest)
        .bind(reserved_size)
        .bind(now_str.clone())
        .bind(expires_at_str)
        .bind(now_str.clone())
        .bind(now_str)
        .bind(UploadStatus::Pending)
        .fetch_one(&mut *tx)
        .await?;

        let reservation = Self::fetch_by_id_or_err(&mut *tx, reservation_id).await?;
        tx.commit().await?;
        Ok((reservation, latest_room))
    }

    async fn fetch_by_id(&self, reservation_id: i64) -> Result<Option<RoomUploadReservation>> {
        Self::fetch_optional(&*self.pool, reservation_id).await
    }

    async fn release_if_pending(&self, reservation_id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        if let Some(reservation) = Self::fetch_optional(&mut *tx, reservation_id).await? {
            let is_pending = reservation
                .upload_status
                .map(|status| matches!(status, UploadStatus::Pending | UploadStatus::Uploading))
                .unwrap_or(true)
                && reservation.consumed_at.is_none();

            if is_pending {
                sqlx::query("DELETE FROM room_upload_reservations WHERE id = ?")
                    .bind(reservation_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query(
                    r#"
                    UPDATE rooms
                    SET current_size = MAX(current_size - ?, 0),
                        updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(reservation.reserved_size)
                .bind(format_naive_datetime(Utc::now().naive_utc()))
                .bind(reservation.room_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn consume_reservation(
        &self,
        reservation_id: i64,
        room_id: i64,
        token_jti: &str,
        actual_size: i64,
        manifest: &str,
    ) -> Result<Room> {
        if actual_size <= 0 {
            return Err(anyhow!("actual uploaded size must be positive"));
        }

        let mut tx = self.pool.begin().await?;
        let reservation = Self::fetch_by_id_or_err(&mut *tx, reservation_id).await?;

        if reservation.room_id != room_id {
            return Err(anyhow!("reservation does not belong to room"));
        }
        if reservation.token_jti != token_jti {
            return Err(anyhow!("reservation token mismatch"));
        }
        if reservation.consumed_at.is_some() {
            return Err(anyhow!("reservation already consumed"));
        }

        let mut room = Self::fetch_room(&mut *tx, room_id).await?;
        let base_size = room
            .current_size
            .checked_sub(reservation.reserved_size)
            .ok_or_else(|| anyhow!("room size underflow while finalizing reservation"))?;
        let new_size = base_size
            .checked_add(actual_size)
            .ok_or_else(|| anyhow!("room size overflow while finalizing reservation"))?;

        if new_size > room.max_size {
            return Err(anyhow!("room size limit exceeded"));
        }

        let now = Utc::now().naive_utc();
        let now_str = format_naive_datetime(now);
        room.current_size = new_size;
        room.updated_at = now;

        sqlx::query(
            r#"
            UPDATE rooms
            SET current_size = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(room.current_size)
        .bind(now_str.clone())
        .bind(room_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE room_upload_reservations
            SET reserved_size = ?,
                file_manifest = ?,
                upload_status = ?,
                consumed_at = ?,
                updated_at = ?,
                chunked_upload = FALSE,
                total_chunks = NULL,
                uploaded_chunks = NULL,
                file_hash = NULL,
                chunk_size = NULL
            WHERE id = ?
            "#,
        )
        .bind(actual_size)
        .bind(manifest)
        .bind(UploadStatus::Completed.to_string())
        .bind(now_str.clone())
        .bind(now_str)
        .bind(reservation_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(room)
    }

    async fn update_uploaded_chunks(
        &self,
        reservation_id: i64,
        uploaded_chunks: i64,
    ) -> Result<RoomUploadReservation> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query(
            r#"
            UPDATE room_upload_reservations
            SET uploaded_chunks = ?,
                upload_status = CASE
                    WHEN upload_status = ? THEN ?
                    ELSE upload_status
                END,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(uploaded_chunks)
        .bind(UploadStatus::Pending.to_string())
        .bind(UploadStatus::Uploading.to_string())
        .bind(now)
        .bind(reservation_id)
        .execute(&mut *tx)
        .await?;

        let updated = Self::fetch_by_id_or_err(&mut *tx, reservation_id).await?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn update_upload_status(
        &self,
        reservation_id: i64,
        status: UploadStatus,
    ) -> Result<RoomUploadReservation> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query(
            r#"
            UPDATE room_upload_reservations
            SET upload_status = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status.to_string())
        .bind(now)
        .bind(reservation_id)
        .execute(&mut *tx)
        .await?;

        let updated = Self::fetch_by_id_or_err(&mut *tx, reservation_id).await?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn consume_upload(&self, reservation_id: i64) -> Result<RoomUploadReservation> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query(
            r#"
            UPDATE room_upload_reservations
            SET consumed_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now.clone())
        .bind(now)
        .bind(reservation_id)
        .execute(&mut *tx)
        .await?;

        let updated = Self::fetch_by_id_or_err(&mut *tx, reservation_id).await?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn mark_uploaded(&self, reservation_id: i64) -> Result<RoomUploadReservation> {
        let mut tx = self.pool.begin().await?;
        let now = format_naive_datetime(Utc::now().naive_utc());
        sqlx::query(
            r#"
            UPDATE room_upload_reservations
            SET upload_status = ?,
                consumed_at = ?,
                updated_at = ?,
                uploaded_chunks = total_chunks
            WHERE id = ?
            "#,
        )
        .bind(UploadStatus::Completed)
        .bind(now.clone())
        .bind(now.clone())
        .bind(reservation_id)
        .execute(&mut *tx)
        .await?;

        let updated = Self::fetch_by_id_or_err(&mut *tx, reservation_id).await?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn delete(&self, reservation_id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM room_upload_reservations WHERE id = ?")
            .bind(reservation_id)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn purge_expired(&self) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM room_upload_reservations WHERE CAST(expires_at AS TEXT) <= ?",
        )
        .bind(format_naive_datetime(Utc::now().naive_utc()))
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
