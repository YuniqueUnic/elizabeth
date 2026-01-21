use std::sync::Arc;

use chrono::{Duration, Utc};
use sqlx::Row;
use tempfile::TempDir;

use crate::config::{AppConfig, AuthConfig};
use crate::db::{DbPoolSettings, init_db, run_migrations};
use crate::models::Room;
use crate::models::RoomToken;
use crate::models::content::{ContentType, RoomContent};
use crate::models::room::row_utils::{format_naive_datetime, parse_any_timestamp};
use crate::repository::{IRoomContentRepository, IRoomRepository, IRoomTokenRepository};
use crate::repository::{RoomContentRepository, RoomRepository, RoomTokenRepository};
use crate::state::AppState;

async fn setup_state(storage_root: &std::path::Path) -> anyhow::Result<Arc<AppState>> {
    let db_settings = DbPoolSettings::new("sqlite::memory:")
        .with_max_connections(1)
        .with_min_connections(1);
    let db_pool = Arc::new(init_db(&db_settings).await?);
    run_migrations(&db_pool, &db_settings.url).await?;

    let mut cfg = AppConfig::for_development();
    cfg.auth = AuthConfig::new("test-secret-key-for-unit-testing-123".to_string())?;
    cfg.storage.root = storage_root.to_path_buf();

    Ok(Arc::new(AppState::new(cfg, db_pool)?))
}

async fn load_gc_markers(
    app_state: &AppState,
    slug: &str,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    let row = sqlx::query(
        r#"
        SELECT CAST(empty_since AS TEXT) as empty_since,
               CAST(cleanup_after AS TEXT) as cleanup_after
        FROM rooms
        WHERE slug = $1
        "#,
    )
    .bind(slug)
    .fetch_one(&*app_state.db_pool)
    .await?;

    let empty_since: Option<String> = row.try_get("empty_since")?;
    let cleanup_after: Option<String> = row.try_get("cleanup_after")?;
    Ok((empty_since, cleanup_after))
}

#[tokio::test]
async fn room_gc_marks_full_unbounded_room_on_empty() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage_root = tmp.path().join("storage");
    let app_state = setup_state(&storage_root).await?;

    let mut room = Room::new("gc-full-unbounded".to_string(), None);
    room.max_times_entered = 1;
    room.current_times_entered = 1;

    let room_repo = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repo.create(&room).await?;
    let room_id = room.id.expect("room id should be set");

    let now = Utc::now().naive_utc();
    let expires_1 = now + Duration::minutes(10);
    let expires_2 = now + Duration::hours(2);

    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    token_repo
        .create(&RoomToken::new(room_id, "t1", expires_1))
        .await?;
    token_repo
        .create(&RoomToken::new(room_id, "t2", expires_2))
        .await?;

    app_state
        .services
        .room_gc
        .on_room_became_empty(&room.slug)
        .await?;

    let (empty_since, cleanup_after) = load_gc_markers(&app_state, &room.slug).await?;
    assert!(empty_since.is_some());
    let cleanup_after = parse_any_timestamp(cleanup_after.as_deref().unwrap().trim())?;
    assert_eq!(cleanup_after, expires_2 + Duration::days(1));

    Ok(())
}

#[tokio::test]
async fn room_gc_clears_markers_when_room_active_again() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage_root = tmp.path().join("storage");
    let app_state = setup_state(&storage_root).await?;

    let mut room = Room::new("gc-clear-markers".to_string(), None);
    room.max_times_entered = 1;
    room.current_times_entered = 1;

    let room_repo = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repo.create(&room).await?;
    let room_id = room.id.expect("room id should be set");

    let expires_at = Utc::now().naive_utc() + Duration::hours(1);
    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    token_repo
        .create(&RoomToken::new(room_id, "t1", expires_at))
        .await?;

    app_state
        .services
        .room_gc
        .on_room_became_empty(&room.slug)
        .await?;
    let (empty_since, cleanup_after) = load_gc_markers(&app_state, &room.slug).await?;
    assert!(empty_since.is_some());
    assert!(cleanup_after.is_some());

    app_state
        .services
        .room_gc
        .on_room_became_active(&room.slug)
        .await?;
    let (empty_since, cleanup_after) = load_gc_markers(&app_state, &room.slug).await?;
    assert!(empty_since.is_none());
    assert!(cleanup_after.is_none());

    Ok(())
}

#[tokio::test]
async fn room_gc_purges_when_cleanup_after_elapsed_and_no_connections() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage_root = tmp.path().join("storage");
    let app_state = setup_state(&storage_root).await?;

    let mut room = Room::new("gc-purge".to_string(), None);
    room.max_times_entered = 1;
    room.current_times_entered = 1;

    let room_repo = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repo.create(&room).await?;
    let room_id = room.id.expect("room id should be set");

    let expires_at = Utc::now().naive_utc() + Duration::hours(1);
    let token_repo = RoomTokenRepository::new(app_state.db_pool.clone());
    token_repo
        .create(&RoomToken::new(room_id, "expired", expires_at))
        .await?;

    app_state
        .services
        .room_gc
        .on_room_became_empty(&room.slug)
        .await?;

    // Force cleanup_after into the past so the room becomes eligible for purge.
    let past = Utc::now().naive_utc() - Duration::hours(1);
    sqlx::query("UPDATE rooms SET cleanup_after = $1 WHERE id = $2")
        .bind(format_naive_datetime(past))
        .bind(room_id)
        .execute(&*app_state.db_pool)
        .await?;

    // Create a file that should be deleted by GC.
    let room_dir = storage_root.join(room_id.to_string());
    tokio::fs::create_dir_all(&room_dir).await?;
    let file_path = room_dir.join("hello.txt");
    tokio::fs::write(&file_path, b"hello").await?;

    let content_repo = RoomContentRepository::new(app_state.db_pool.clone());
    let now = Utc::now().naive_utc();
    let mut content = RoomContent::builder()
        .room_id(room_id)
        .content_type(ContentType::File)
        .now(now)
        .build();
    content.set_path(
        file_path.to_string_lossy().to_string(),
        ContentType::File,
        5,
        "text/plain".to_string(),
    );
    content.file_name = Some("hello.txt".to_string());
    content_repo.create(&content).await?;

    let cleaned = app_state
        .services
        .room_gc
        .run_scheduled_gc(&app_state.connection_manager, 10)
        .await?;
    assert_eq!(cleaned, 1);

    assert!(!file_path.exists());
    assert!(!room_dir.exists());

    let room = room_repo.find_by_id(room_id).await?;
    assert!(room.is_none());

    Ok(())
}
