use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{Duration, NaiveDateTime, Utc};

use crate::repository::RoomLifecycleRepository;
use crate::websocket::connection::ConnectionManager;

const FULL_ROOM_TOKEN_GRACE_PERIOD: Duration = Duration::days(1);

#[derive(Debug, Clone, Default)]
pub struct RoomLifecycleReport {
    pub expired_rooms: u64,
    pub full_rooms: u64,
    pub released_names: u64,
}

impl RoomLifecycleReport {
    pub fn changed(&self) -> u64 {
        self.expired_rooms + self.full_rooms + self.released_names
    }
}

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
pub struct RoomLifecycleService {
    repository: Arc<RoomLifecycleRepository>,
    storage_root: PathBuf,
}

impl RoomLifecycleService {
    pub fn new(repository: Arc<RoomLifecycleRepository>, storage_root: PathBuf) -> Self {
        Self {
            repository,
            storage_root,
        }
    }

    pub async fn on_room_became_active(&self, room_slug: &str) -> Result<()> {
        self.repository.clear_cleanup_markers(room_slug).await
    }

    pub async fn on_room_became_empty(&self, room_slug: &str) -> Result<()> {
        let Some(state) = self.repository.load_empty_state(room_slug).await? else {
            return Ok(());
        };
        if state.expire_at.is_some() || state.current_times_entered < state.max_times_entered {
            return self.repository.clear_cleanup_markers(room_slug).await;
        }

        let now = Utc::now().naive_utc();
        let cleanup_after =
            state.max_token_expires_at.unwrap_or(now) + FULL_ROOM_TOKEN_GRACE_PERIOD;
        self.repository
            .mark_empty(state.id, now, cleanup_after)
            .await
    }

    pub async fn list_full_unbounded_rooms(
        &self,
        manager: &ConnectionManager,
        limit: u32,
    ) -> Result<Vec<FullRoomGcStatus>> {
        let rooms = self.repository.list_full_unbounded(limit).await?;
        let mut result = Vec::with_capacity(rooms.len());
        for room in rooms {
            result.push(FullRoomGcStatus {
                active_connections: manager.get_room_connection_count(&room.slug).await,
                id: room.id,
                name: room.name,
                slug: room.slug,
                max_times_entered: room.max_times_entered,
                current_times_entered: room.current_times_entered,
                empty_since: room.empty_since,
                cleanup_after: room.cleanup_after,
                max_token_expires_at: room.max_token_expires_at,
            });
        }
        Ok(result)
    }

    pub async fn run(
        &self,
        manager: &ConnectionManager,
        batch_limit: u32,
        private_name_lock_seconds: i64,
    ) -> Result<RoomLifecycleReport> {
        let now = Utc::now().naive_utc();
        let expired_rooms = self.purge_expired(manager, now, batch_limit).await?;
        let full_rooms = self.purge_full(manager, now, batch_limit).await?;
        let threshold = now - Duration::seconds(private_name_lock_seconds.max(0));
        let released_names = self
            .repository
            .release_private_names(now, threshold)
            .await?;
        Ok(RoomLifecycleReport {
            expired_rooms,
            full_rooms,
            released_names,
        })
    }

    pub async fn delete_room(
        &self,
        manager: &ConnectionManager,
        room_id: i64,
        slug: &str,
    ) -> Result<bool> {
        manager.disconnect_room(slug, "Room deleted").await;
        self.purge_room(room_id).await
    }

    async fn purge_expired(
        &self,
        manager: &ConnectionManager,
        now: NaiveDateTime,
        limit: u32,
    ) -> Result<u64> {
        let candidates = self.repository.list_expired_due(now, limit).await?;
        let mut cleaned = 0;
        for candidate in candidates {
            manager
                .disconnect_room(&candidate.slug, "Room expired")
                .await;
            if self.purge_room(candidate.id).await? {
                cleaned += 1;
            }
        }
        Ok(cleaned)
    }

    async fn purge_full(
        &self,
        manager: &ConnectionManager,
        now: NaiveDateTime,
        limit: u32,
    ) -> Result<u64> {
        let candidates = self.repository.list_full_due(now, limit).await?;
        let mut cleaned = 0;
        for candidate in candidates {
            if manager.get_room_connection_count(&candidate.slug).await > 0 {
                self.repository
                    .clear_cleanup_markers(&candidate.slug)
                    .await?;
                continue;
            }
            let Some(room) = self.repository.find_room(candidate.id).await? else {
                continue;
            };
            if room.expire_at.is_some() || room.current_times_entered < room.max_times_entered {
                self.repository
                    .clear_cleanup_markers(&candidate.slug)
                    .await?;
                continue;
            }
            if self.purge_room(candidate.id).await? {
                cleaned += 1;
            }
        }
        Ok(cleaned)
    }

    async fn purge_room(&self, room_id: i64) -> Result<bool> {
        for path in self.repository.list_content_paths(room_id).await? {
            remove_file_if_present(Path::new(&path)).await?;
        }
        remove_dir_if_present(&self.storage_root.join(room_id.to_string())).await?;
        self.repository
            .delete_room_graph(room_id)
            .await
            .context("failed to delete room persistence graph")
    }
}

async fn remove_file_if_present(path: &Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to remove {}", path.display())),
    }
}

async fn remove_dir_if_present(path: &Path) -> Result<()> {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to remove {}", path.display())),
    }
}
