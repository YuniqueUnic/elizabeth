use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use sqlx::Row;

use crate::db::DbPool;
use crate::models::Room;
use crate::models::room::row_utils::{format_naive_datetime, parse_any_timestamp};

#[derive(Debug, Clone)]
pub struct RoomLifecycleCandidate {
    pub id: i64,
    pub slug: String,
}

#[derive(Debug, Clone)]
pub struct EmptyRoomState {
    pub id: i64,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
    pub expire_at: Option<NaiveDateTime>,
    pub max_token_expires_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct FullRoomState {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
    pub empty_since: Option<NaiveDateTime>,
    pub cleanup_after: Option<NaiveDateTime>,
    pub max_token_expires_at: Option<NaiveDateTime>,
}

#[derive(Clone)]
pub struct RoomLifecycleRepository {
    pool: Arc<DbPool>,
}

impl RoomLifecycleRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    pub async fn clear_cleanup_markers(&self, slug: &str) -> Result<()> {
        sqlx::query("UPDATE rooms SET empty_since = NULL, cleanup_after = NULL WHERE slug = $1")
            .bind(slug)
            .execute(&*self.pool)
            .await
            .context("failed to clear room lifecycle markers")?;
        Ok(())
    }

    pub async fn load_empty_state(&self, slug: &str) -> Result<Option<EmptyRoomState>> {
        let row = sqlx::query(
            r#"
            SELECT id, max_times_entered, current_times_entered,
                   CAST(expire_at AS TEXT) AS expire_at,
                   (SELECT MAX(CAST(expires_at AS TEXT))
                    FROM room_tokens
                    WHERE room_id = rooms.id AND revoked_at IS NULL) AS max_token_expires_at
            FROM rooms
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&*self.pool)
        .await
        .context("failed to load empty room lifecycle state")?;

        row.map(|row| {
            Ok(EmptyRoomState {
                id: row.try_get("id")?,
                max_times_entered: row.try_get("max_times_entered")?,
                current_times_entered: row.try_get("current_times_entered")?,
                expire_at: parse_optional_timestamp(row.try_get("expire_at")?),
                max_token_expires_at: parse_optional_timestamp(
                    row.try_get("max_token_expires_at")?,
                ),
            })
        })
        .transpose()
    }

    pub async fn mark_empty(
        &self,
        room_id: i64,
        empty_since: NaiveDateTime,
        cleanup_after: NaiveDateTime,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE rooms
            SET empty_since = $1, cleanup_after = $2
            WHERE id = $3
              AND expire_at IS NULL
              AND current_times_entered >= max_times_entered
            "#,
        )
        .bind(format_naive_datetime(empty_since))
        .bind(format_naive_datetime(cleanup_after))
        .bind(room_id)
        .execute(&*self.pool)
        .await
        .context("failed to mark room empty for lifecycle cleanup")?;
        Ok(())
    }

    pub async fn list_full_unbounded(&self, limit: u32) -> Result<Vec<FullRoomState>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, slug, max_times_entered, current_times_entered,
                   CAST(empty_since AS TEXT) AS empty_since,
                   CAST(cleanup_after AS TEXT) AS cleanup_after,
                   (SELECT MAX(CAST(expires_at AS TEXT))
                    FROM room_tokens
                    WHERE room_id = rooms.id AND revoked_at IS NULL) AS max_token_expires_at
            FROM rooms
            WHERE expire_at IS NULL
              AND current_times_entered >= max_times_entered
            ORDER BY updated_at DESC
            LIMIT $1
            "#,
        )
        .bind(i64::from(limit))
        .fetch_all(&*self.pool)
        .await
        .context("failed to list full unbounded rooms")?;

        rows.into_iter()
            .map(|row| {
                Ok(FullRoomState {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    slug: row.try_get("slug")?,
                    max_times_entered: row.try_get("max_times_entered")?,
                    current_times_entered: row.try_get("current_times_entered")?,
                    empty_since: parse_optional_timestamp(row.try_get("empty_since")?),
                    cleanup_after: parse_optional_timestamp(row.try_get("cleanup_after")?),
                    max_token_expires_at: parse_optional_timestamp(
                        row.try_get("max_token_expires_at")?,
                    ),
                })
            })
            .collect()
    }

    pub async fn list_expired_due(
        &self,
        now: NaiveDateTime,
        limit: u32,
    ) -> Result<Vec<RoomLifecycleCandidate>> {
        self.list_candidates("expire_at IS NOT NULL AND expire_at <= $1", now, limit)
            .await
    }

    pub async fn list_full_due(
        &self,
        now: NaiveDateTime,
        limit: u32,
    ) -> Result<Vec<RoomLifecycleCandidate>> {
        self.list_candidates(
            "expire_at IS NULL AND current_times_entered >= max_times_entered AND empty_since IS NOT NULL AND cleanup_after IS NOT NULL AND cleanup_after <= $1",
            now,
            limit,
        )
        .await
    }

    async fn list_candidates(
        &self,
        predicate: &str,
        now: NaiveDateTime,
        limit: u32,
    ) -> Result<Vec<RoomLifecycleCandidate>> {
        let sql = format!(
            "SELECT id, slug FROM rooms WHERE {predicate} ORDER BY updated_at ASC LIMIT $2"
        );
        let rows = sqlx::query(&sql)
            .bind(format_naive_datetime(now))
            .bind(i64::from(limit))
            .fetch_all(&*self.pool)
            .await
            .context("failed to list due room lifecycle candidates")?;
        rows.into_iter()
            .map(|row| {
                Ok(RoomLifecycleCandidate {
                    id: row.try_get("id")?,
                    slug: row.try_get("slug")?,
                })
            })
            .collect()
    }

    pub async fn find_room(&self, room_id: i64) -> Result<Option<Room>> {
        sqlx::query_as::<_, Room>(
            r#"
            SELECT id, name, slug, password, status, max_size, current_size,
                   max_times_entered, current_times_entered,
                   CAST(expire_at AS TEXT) AS expire_at,
                   CAST(created_at AS TEXT) AS created_at,
                   CAST(updated_at AS TEXT) AS updated_at,
                   permission
            FROM rooms WHERE id = $1
            "#,
        )
        .bind(room_id)
        .fetch_optional(&*self.pool)
        .await
        .context("failed to reload room lifecycle candidate")
    }

    pub async fn list_content_paths(&self, room_id: i64) -> Result<Vec<String>> {
        sqlx::query_scalar("SELECT path FROM room_contents WHERE room_id = $1 AND path IS NOT NULL")
            .bind(room_id)
            .fetch_all(&*self.pool)
            .await
            .context("failed to list room storage paths")
    }

    pub async fn delete_room_graph(&self, room_id: i64) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "DELETE FROM room_chunk_uploads WHERE reservation_id IN (SELECT id FROM room_upload_reservations WHERE room_id = $1)",
        )
        .bind(room_id)
        .execute(&mut *tx)
        .await?;
        for table in [
            "room_upload_reservations",
            "room_refresh_tokens",
            "room_tokens",
            "room_contents",
            "room_access_logs",
        ] {
            let sql = format!("DELETE FROM {table} WHERE room_id = $1");
            sqlx::query(&sql).bind(room_id).execute(&mut *tx).await?;
        }
        let deleted = sqlx::query("DELETE FROM rooms WHERE id = $1")
            .bind(room_id)
            .execute(&mut *tx)
            .await?
            .rows_affected()
            > 0;
        tx.commit().await?;
        Ok(deleted)
    }

    pub async fn release_private_names(
        &self,
        now: NaiveDateTime,
        threshold: NaiveDateTime,
    ) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE rooms SET name = slug, updated_at = $1 WHERE slug != name AND updated_at <= $2",
        )
        .bind(format_naive_datetime(now))
        .bind(format_naive_datetime(threshold))
        .execute(&*self.pool)
        .await
        .context("failed to release expired private room names")?;
        Ok(result.rows_affected())
    }
}

fn parse_optional_timestamp(value: Option<String>) -> Option<NaiveDateTime> {
    value
        .as_deref()
        .and_then(|value| parse_any_timestamp(value.trim()).ok())
}
