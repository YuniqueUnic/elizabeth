use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{Duration, NaiveDateTime, Utc};
use sqlx::Row;

use crate::db::DbPool;
use crate::models::room::row_utils::{format_naive_datetime, parse_any_timestamp};
use crate::repository::{
    IRoomContentRepository, IRoomRepository, IRoomTokenRepository, RoomContentRepository,
    RoomRepository, RoomTokenRepository,
};
use crate::websocket::connection::ConnectionManager;

const FULL_ROOM_TOKEN_GRACE_PERIOD: Duration = Duration::days(1);

#[derive(Debug, Clone)]
pub struct FullRoomGcStatus {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
    pub empty_since: Option<NaiveDateTime>,
    pub cleanup_after: Option<NaiveDateTime>,
    pub max_token_expires_at: Option<NaiveDateTime>,
    pub active_connections: usize,
}

#[derive(Clone)]
pub struct RoomGcService {
    db_pool: Arc<DbPool>,
    storage_root: PathBuf,
}

impl RoomGcService {
    pub fn new(db_pool: Arc<DbPool>, storage_root: PathBuf) -> Self {
        Self {
            db_pool,
            storage_root,
        }
    }

    pub async fn on_room_became_active(&self, room_slug: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE rooms
            SET empty_since = NULL, cleanup_after = NULL
            WHERE slug = $1
            "#,
        )
        .bind(room_slug)
        .execute(&*self.db_pool)
        .await
        .context("failed to clear room gc markers")?;
        Ok(())
    }

    pub async fn on_room_became_empty(&self, room_slug: &str) -> Result<()> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                max_times_entered,
                current_times_entered,
                CAST(expire_at AS TEXT) as expire_at
            FROM rooms
            WHERE slug = $1
            "#,
        )
        .bind(room_slug)
        .fetch_optional(&*self.db_pool)
        .await
        .context("failed to load room for gc")?;

        let Some(row) = row else {
            return Ok(());
        };

        let room_id: i64 = row.try_get("id")?;
        let max_times_entered: i64 = row.try_get("max_times_entered")?;
        let current_times_entered: i64 = row.try_get("current_times_entered")?;
        let expire_at_raw: Option<String> = row.try_get("expire_at")?;
        let expire_at = expire_at_raw
            .as_deref()
            .and_then(|value| parse_any_timestamp(value.trim()).ok());

        let is_unbounded = expire_at.is_none();
        let is_full = current_times_entered >= max_times_entered;

        if !is_unbounded || !is_full {
            self.on_room_became_active(room_slug).await?;
            return Ok(());
        }

        let max_token_expires_at_raw: Option<String> = sqlx::query_scalar(
            r#"
            SELECT MAX(CAST(expires_at AS TEXT))
            FROM room_tokens
            WHERE room_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(room_id)
        .fetch_one(&*self.db_pool)
        .await
        .context("failed to compute room max token expires_at")?;

        let max_token_expires_at = max_token_expires_at_raw
            .as_deref()
            .and_then(|value| parse_any_timestamp(value.trim()).ok())
            .unwrap_or_else(|| Utc::now().naive_utc());

        let cleanup_after = max_token_expires_at + FULL_ROOM_TOKEN_GRACE_PERIOD;
        let empty_since = Utc::now().naive_utc();
        let empty_since_str = format_naive_datetime(empty_since);
        let cleanup_after_str = format_naive_datetime(cleanup_after);

        sqlx::query(
            r#"
            UPDATE rooms
            SET empty_since = $1, cleanup_after = $2
            WHERE id = $3
              AND expire_at IS NULL
              AND current_times_entered >= max_times_entered
            "#,
        )
        .bind(empty_since_str)
        .bind(cleanup_after_str)
        .bind(room_id)
        .execute(&*self.db_pool)
        .await
        .context("failed to mark room as empty for gc")?;

        Ok(())
    }

    pub async fn list_full_unbounded_rooms(
        &self,
        manager: &ConnectionManager,
        limit: u32,
    ) -> Result<Vec<FullRoomGcStatus>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                name,
                slug,
                max_times_entered,
                current_times_entered,
                CAST(empty_since AS TEXT) as empty_since,
                CAST(cleanup_after AS TEXT) as cleanup_after,
                (
                    SELECT MAX(CAST(expires_at AS TEXT))
                    FROM room_tokens
                    WHERE room_id = rooms.id AND revoked_at IS NULL
                ) as max_token_expires_at
            FROM rooms
            WHERE expire_at IS NULL
              AND current_times_entered >= max_times_entered
            ORDER BY updated_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&*self.db_pool)
        .await
        .context("failed to list full unbounded rooms")?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.try_get("id")?;
            let name: String = row.try_get("name")?;
            let slug: String = row.try_get("slug")?;
            let max_times_entered: i64 = row.try_get("max_times_entered")?;
            let current_times_entered: i64 = row.try_get("current_times_entered")?;

            let empty_since_raw: Option<String> = row.try_get("empty_since")?;
            let cleanup_after_raw: Option<String> = row.try_get("cleanup_after")?;
            let max_token_expires_at_raw: Option<String> = row.try_get("max_token_expires_at")?;

            let empty_since = empty_since_raw
                .as_deref()
                .and_then(|value| parse_any_timestamp(value.trim()).ok());
            let cleanup_after = cleanup_after_raw
                .as_deref()
                .and_then(|value| parse_any_timestamp(value.trim()).ok());
            let max_token_expires_at = max_token_expires_at_raw
                .as_deref()
                .and_then(|value| parse_any_timestamp(value.trim()).ok());

            let active_connections = manager.get_room_connection_count(&slug).await;

            result.push(FullRoomGcStatus {
                id,
                name,
                slug,
                max_times_entered,
                current_times_entered,
                empty_since,
                cleanup_after,
                max_token_expires_at,
                active_connections,
            });
        }

        Ok(result)
    }

    pub async fn run_scheduled_gc(&self, manager: &ConnectionManager, limit: u32) -> Result<u64> {
        let now = Utc::now().naive_utc();
        let now_str = format_naive_datetime(now);

        let rows = sqlx::query(
            r#"
            SELECT id, slug
            FROM rooms
            WHERE expire_at IS NULL
              AND current_times_entered >= max_times_entered
              AND empty_since IS NOT NULL
              AND cleanup_after IS NOT NULL
              AND CAST(cleanup_after AS TEXT) <= $1
            ORDER BY cleanup_after ASC
            LIMIT $2
            "#,
        )
        .bind(now_str)
        .bind(limit as i64)
        .fetch_all(&*self.db_pool)
        .await
        .context("failed to query rooms eligible for gc")?;

        if rows.is_empty() {
            return Ok(0);
        }

        let room_repo = RoomRepository::new(self.db_pool.clone());
        let content_repo = RoomContentRepository::new(self.db_pool.clone());
        let token_repo = RoomTokenRepository::new(self.db_pool.clone());

        let mut cleaned = 0u64;
        for row in rows {
            let room_id: i64 = row.try_get("id")?;
            let slug: String = row.try_get("slug")?;

            if manager.get_room_connection_count(&slug).await > 0 {
                // Room became active again; cancel cleanup markers.
                self.on_room_became_active(&slug).await?;
                continue;
            }

            let room = room_repo
                .find_by_id(room_id)
                .await
                .context("failed to reload room")?;
            let Some(room) = room else {
                continue;
            };

            if room.expire_at.is_some() || room.current_times_entered < room.max_times_entered {
                self.on_room_became_active(&slug).await?;
                continue;
            }

            // Delete files on disk first.
            let contents = content_repo
                .list_by_room(room_id)
                .await
                .context("failed to list room contents for gc")?;
            for content in &contents {
                if let Some(path) = &content.path {
                    tokio::fs::remove_file(path).await.ok();
                }
            }

            // Best-effort remove room directory (may be empty / may not exist).
            let room_dir = self.storage_root.join(room_id.to_string());
            let _ = tokio::fs::remove_dir_all(&room_dir).await;

            // Remove DB content rows (room delete cascades, but we want to avoid leaving orphan files if cascade fails).
            content_repo
                .delete_by_room_id(room_id)
                .await
                .context("failed to delete room contents for gc")?;

            // Remove tokens explicitly for clarity (room delete should cascade).
            token_repo
                .delete_by_room(room_id)
                .await
                .context("failed to delete room tokens for gc")?;

            // Finally delete the room row.
            let deleted = room_repo
                .delete(&room.slug)
                .await
                .context("failed to delete room for gc")?;
            if deleted {
                cleaned += 1;
            }
        }

        Ok(cleaned)
    }
}
