use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use chrono::{Duration, Utc};
use sqlx::Row;
use tempfile::TempDir;
use tokio::sync::Barrier;

use crate::config::{AppConfig, AuthConfig};
use crate::db::{DbPoolSettings, init_db, run_migrations};
use crate::dto::rooms::{RoomView, VerifyRoomPasswordRequest};
use crate::handlers::rooms::find;
use crate::handlers::rooms::tokens::{issue_token, verify_password};
use crate::models::content::{ContentType, RoomContent};
use crate::models::{Room, RoomRefreshToken, RoomToken, permission::RoomPermission};
use crate::repository::{
    IRoomContentRepository, IRoomRefreshTokenRepository, IRoomRepository, IRoomTokenRepository,
    IRoomUploadReservationRepository, RoomAccessRepository, RoomContentRepository,
    RoomRefreshTokenRepository, RoomRepository, RoomTokenRepository,
    RoomUploadReservationRepository,
};
use crate::scheduler::ScheduledTask;
use crate::services::{RoomPasswordService, migrate_legacy_room_passwords};
use crate::state::AppState;
use crate::tasks::UploadCleanupTask;
use crate::websocket::handler::MessageHandler;
use crate::websocket::types::ConnectRequest;

async fn setup_state(storage_root: &std::path::Path) -> anyhow::Result<Arc<AppState>> {
    let settings = DbPoolSettings::new("sqlite::memory:")
        .with_max_connections(1)
        .with_min_connections(1);
    let pool = Arc::new(init_db(&settings).await?);
    run_migrations(&pool, &settings.url).await?;
    let mut config = AppConfig::for_development();
    config.auth = AuthConfig::new("test-secret-key-for-room-policy-tests-123".to_string())?;
    config.storage.root = storage_root.to_path_buf();
    Ok(Arc::new(AppState::new(config, pool)?))
}

fn future_room(name: &str) -> Room {
    let mut room = Room::new(name.to_string(), None);
    room.expire_at = Some(Utc::now().naive_utc() + Duration::days(7));
    room
}

#[tokio::test]
async fn legacy_passwords_are_migrated_and_room_views_never_leak_them() -> anyhow::Result<()> {
    let state = setup_state(std::env::temp_dir().as_path()).await?;
    let mut room = future_room("legacy-password-room");
    room.password = Some("legacy-plaintext".to_string());
    let room = state.services.room_repository.create(&room).await?;

    let migrated = migrate_legacy_room_passwords(&state.db_pool, &RoomPasswordService).await?;
    assert_eq!(migrated, 1);

    let stored = state
        .services
        .room_repository
        .find_by_id(room.id.unwrap())
        .await?
        .unwrap();
    let encoded = stored.password.as_deref().unwrap();
    assert!(encoded.starts_with("$argon2"));
    assert!(
        state
            .room_password_service()
            .verify("legacy-plaintext".to_string(), encoded.to_string())
            .await?
    );

    let json = serde_json::to_value(RoomView::from(&stored))?;
    assert_eq!(json["password_protected"], true);
    assert!(json.get("password").is_none());
    assert!(!json.to_string().contains("legacy-plaintext"));
    assert!(!json.to_string().contains("argon2"));
    Ok(())
}

#[tokio::test]
async fn password_verification_does_not_consume_quota_or_create_tokens() -> anyhow::Result<()> {
    let state = setup_state(std::env::temp_dir().as_path()).await?;
    let mut room = future_room("password-side-effect-room");
    room.password = Some(
        state
            .room_password_service()
            .hash("correct-password".to_string())
            .await?,
    );
    let room = state.services.room_repository.create(&room).await?;

    let _response = verify_password(
        Path(room.slug.clone()),
        State(state.clone()),
        Json(VerifyRoomPasswordRequest {
            password: "correct-password".to_string(),
        }),
    )
    .await?;

    let current_times: i64 =
        sqlx::query_scalar("SELECT current_times_entered FROM rooms WHERE id = $1")
            .bind(room.id.unwrap())
            .fetch_one(&*state.db_pool)
            .await?;
    let token_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM room_tokens WHERE room_id = $1")
            .bind(room.id.unwrap())
            .fetch_one(&*state.db_pool)
            .await?;
    assert_eq!(current_times, 0);
    assert_eq!(token_count, 0);
    Ok(())
}

#[tokio::test]
async fn concurrent_session_grants_stop_exactly_at_the_room_limit() -> anyhow::Result<()> {
    let temp = tempfile::NamedTempFile::new()?;
    let url = format!("sqlite://{}?mode=rwc", temp.path().display());
    let settings = DbPoolSettings::new(url.clone())
        .with_max_connections(8)
        .with_min_connections(1);
    let pool = Arc::new(init_db(&settings).await?);
    run_migrations(&pool, &url).await?;
    let mut room = future_room("concurrent-quota-room");
    room.max_times_entered = 3;
    let room = RoomRepository::new(pool.clone()).create(&room).await?;
    let room_id = room.id.unwrap();
    let barrier = Arc::new(Barrier::new(11));
    let mut handles = Vec::new();

    for index in 0..10 {
        let repository = RoomAccessRepository::new(pool.clone());
        let barrier = barrier.clone();
        handles.push(tokio::spawn(async move {
            let token = RoomToken::new(
                room_id,
                format!("concurrent-{index}"),
                Utc::now().naive_utc() + Duration::hours(1),
            );
            barrier.wait().await;
            repository
                .grant_new_session(room_id, &token, None, Utc::now().naive_utc())
                .await
        }));
    }
    barrier.wait().await;

    let mut successes = 0;
    for handle in handles {
        if handle.await?? {
            successes += 1;
        }
    }
    assert_eq!(successes, 3);
    let stored_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM room_tokens WHERE room_id = $1")
            .bind(room_id)
            .fetch_one(&*pool)
            .await?;
    let entered: i64 = sqlx::query_scalar("SELECT current_times_entered FROM rooms WHERE id = $1")
        .bind(room_id)
        .fetch_one(&*pool)
        .await?;
    assert_eq!(stored_count, 3);
    assert_eq!(entered, 3);
    Ok(())
}

#[tokio::test]
async fn refresh_uses_live_room_policy_and_persists_the_new_access_token() -> anyhow::Result<()> {
    let state = setup_state(std::env::temp_dir().as_path()).await?;
    let room = state
        .services
        .room_repository
        .create(&future_room("refresh-live-policy"))
        .await?;
    let (access_token, claims) = state.token_service().issue(&room)?;
    let prepared = state
        .refresh_token_service()
        .prepare_refresh_token(&room, claims.jti.clone())?;
    let access_record = RoomToken::new(room.id.unwrap(), claims.jti.clone(), claims.expires_at());
    assert!(
        RoomAccessRepository::new(state.db_pool.clone())
            .grant_new_session(
                room.id.unwrap(),
                &access_record,
                Some(&prepared.record),
                Utc::now().naive_utc(),
            )
            .await?
    );

    let mut live_room = room.clone();
    live_room.permission = RoomPermission::VIEW_ONLY;
    state
        .services
        .room_repository
        .update_permissions_and_slug(&live_room)
        .await?;

    let refreshed = state
        .refresh_token_service()
        .refresh_access_token(&prepared.signed_token)
        .await?;
    let refreshed_claims = state.token_service().decode(&refreshed.access_token)?;
    assert_eq!(
        refreshed_claims.permission,
        RoomPermission::VIEW_ONLY.bits()
    );
    assert_ne!(refreshed.access_token, access_token);

    let token_repository = RoomTokenRepository::new(state.db_pool.clone());
    assert!(
        token_repository
            .find_by_jti(&refreshed_claims.jti)
            .await?
            .is_some_and(|token| token.is_active())
    );
    assert!(
        !token_repository
            .find_by_jti(&claims.jti)
            .await?
            .unwrap()
            .is_active()
    );
    Ok(())
}

#[tokio::test]
async fn websocket_handshake_rejects_revoked_sessions_and_returns_live_room_info()
-> anyhow::Result<()> {
    let state = setup_state(std::env::temp_dir().as_path()).await?;
    let room = state
        .services
        .room_repository
        .create(&future_room("websocket-policy-room"))
        .await?;
    let Json(issued) = issue_token(
        Path(room.slug.clone()),
        State(state.clone()),
        Json(crate::dto::rooms::IssueTokenRequest {
            password: None,
            token: None,
            with_refresh_token: false,
        }),
    )
    .await?;
    let handler = MessageHandler::new((*state).clone(), state.connection_manager.clone());
    let request = ConnectRequest {
        token: issued.token.clone(),
        room_name: room.slug.clone(),
    };
    let connected = handler.handle_connect(request.clone()).await?;
    assert_eq!(connected.room_info.unwrap().id, room.id.unwrap());

    RoomTokenRepository::new(state.db_pool.clone())
        .revoke(&issued.claims.jti)
        .await?;
    assert!(handler.handle_connect(request).await.is_err());
    Ok(())
}

#[tokio::test]
async fn expired_room_returns_gone_without_query_side_effects_or_recreation() -> anyhow::Result<()>
{
    let state = setup_state(std::env::temp_dir().as_path()).await?;
    let room = state
        .services
        .room_repository
        .create(&future_room("expired-gone-room"))
        .await?;
    sqlx::query("UPDATE rooms SET expire_at = $1 WHERE id = $2")
        .bind(crate::models::room::row_utils::format_naive_datetime(
            Utc::now().naive_utc() - Duration::minutes(1),
        ))
        .bind(room.id.unwrap())
        .execute(&*state.db_pool)
        .await?;

    let error = find(Path(room.slug.clone()), State(state.clone()))
        .await
        .expect_err("expired room should return 410");
    assert_eq!(error.status_code(), axum::http::StatusCode::GONE);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE id = $1")
        .bind(room.id.unwrap())
        .fetch_one(&*state.db_pool)
        .await?;
    assert_eq!(count, 1, "read path must not delete or recreate the room");
    Ok(())
}

#[tokio::test]
async fn lifecycle_cleanup_removes_expired_room_storage_and_persistence_graph() -> anyhow::Result<()>
{
    let temp = TempDir::new()?;
    let state = setup_state(&temp.path().join("storage")).await?;
    let room = state
        .services
        .room_repository
        .create(&future_room("expired-cleanup-room"))
        .await?;
    let room_id = room.id.unwrap();

    let room_dir = state.storage_root().join(room_id.to_string());
    tokio::fs::create_dir_all(&room_dir).await?;
    let file_path = room_dir.join("payload.txt");
    tokio::fs::write(&file_path, b"payload").await?;
    let now = Utc::now().naive_utc();
    let mut content = RoomContent::builder()
        .room_id(room_id)
        .content_type(ContentType::File)
        .sequence_number(0)
        .now(now)
        .build();
    content.set_path(
        file_path.to_string_lossy().to_string(),
        ContentType::File,
        7,
        "text/plain".to_string(),
    );
    content.file_name = Some("payload.txt".to_string());
    RoomContentRepository::new(state.db_pool.clone())
        .create(&content)
        .await?;
    RoomTokenRepository::new(state.db_pool.clone())
        .create(&RoomToken::new(
            room_id,
            "cleanup-access",
            now + Duration::hours(1),
        ))
        .await?;
    RoomRefreshTokenRepository::new(state.db_pool.clone())
        .create(&RoomRefreshToken::new(
            room_id,
            "cleanup-access".to_string(),
            "cleanup-refresh",
            now + Duration::hours(2),
        ))
        .await?;
    RoomUploadReservationRepository::new(state.db_pool.clone())
        .reserve_upload(
            &room,
            "cleanup-upload",
            "cleanup-access",
            "[]",
            1,
            Duration::hours(1),
        )
        .await?;
    sqlx::query("UPDATE rooms SET expire_at = $1 WHERE id = $2")
        .bind(crate::models::room::row_utils::format_naive_datetime(
            now - Duration::minutes(1),
        ))
        .bind(room_id)
        .execute(&*state.db_pool)
        .await?;

    let report = state
        .services
        .room_lifecycle
        .run(
            &state.connection_manager,
            10,
            state.config.room.share_disabled_lock_duration,
        )
        .await?;
    assert_eq!(report.expired_rooms, 1);
    assert!(!file_path.exists());
    assert!(!room_dir.exists());
    for table in [
        "rooms",
        "room_contents",
        "room_tokens",
        "room_refresh_tokens",
        "room_upload_reservations",
    ] {
        let sql = format!(
            "SELECT COUNT(*) FROM {table} WHERE {} = $1",
            if table == "rooms" { "id" } else { "room_id" }
        );
        let count: i64 = sqlx::query_scalar(&sql)
            .bind(room_id)
            .fetch_one(&*state.db_pool)
            .await?;
        assert_eq!(count, 0, "{table} should be cleaned");
    }
    Ok(())
}

#[tokio::test]
async fn legacy_expiry_migration_backfills_one_week_and_adds_upload_owner() -> anyhow::Result<()> {
    let pool = init_db(
        &DbPoolSettings::new("sqlite::memory:")
            .with_max_connections(1)
            .with_min_connections(1),
    )
    .await?;
    sqlx::raw_sql(
        r#"
        CREATE TABLE rooms (
            id INTEGER PRIMARY KEY,
            expire_at DATETIME,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL
        );
        CREATE TABLE room_upload_reservations (id INTEGER PRIMARY KEY, room_id INTEGER NOT NULL);
        INSERT INTO rooms (id, expire_at, created_at, updated_at)
        VALUES (1, NULL, '2026-01-01 00:00:00', '2026-01-01 00:00:00');
        "#,
    )
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../migrations/004_room_expiry_backfill.sql"
    ))
    .execute(&pool)
    .await?;

    let expire_at: String =
        sqlx::query_scalar("SELECT CAST(expire_at AS TEXT) FROM rooms WHERE id = 1")
            .fetch_one(&pool)
            .await?;
    assert!(expire_at.starts_with("2026-01-08 00:00:00"));
    let column_count: i64 = sqlx::query("PRAGMA table_info(room_upload_reservations)")
        .fetch_all(&pool)
        .await?
        .into_iter()
        .filter(|row| {
            row.try_get::<String, _>("name")
                .is_ok_and(|name| name == "owner_token_jti")
        })
        .count() as i64;
    assert_eq!(column_count, 1);
    Ok(())
}

#[tokio::test]
async fn upload_cleanup_removes_expired_chunk_files_before_database_rows() -> anyhow::Result<()> {
    let state = setup_state(std::env::temp_dir().as_path()).await?;
    let room = state
        .services
        .room_repository
        .create(&future_room("expired-chunk-cleanup-room"))
        .await?;
    let repository = Arc::new(RoomUploadReservationRepository::new(state.db_pool.clone()));
    let (reservation, _) = repository
        .reserve_upload(
            &room,
            "expired-upload-token",
            "owner-jti",
            "[]",
            8,
            Duration::hours(1),
        )
        .await?;
    let reservation_id = reservation.id.expect("reservation id");
    sqlx::query(
        r#"
        UPDATE room_upload_reservations
        SET chunked_upload = true,
            total_chunks = 1,
            chunk_size = 8,
            reserved_at = datetime('now', '-2 hours'),
            expires_at = datetime('now', '-1 hour')
        WHERE id = $1
        "#,
    )
    .bind(reservation_id)
    .execute(&*state.db_pool)
    .await?;

    let chunk_dir = crate::chunk_temp_storage::reservation_dir(reservation_id);
    tokio::fs::create_dir_all(&chunk_dir).await?;
    tokio::fs::write(chunk_dir.join("chunk_0"), b"orphaned").await?;

    let report = UploadCleanupTask::new(repository.clone()).run().await?;
    assert_eq!(report.changed, 1);
    assert!(!tokio::fs::try_exists(&chunk_dir).await?);
    assert!(repository.fetch_by_id(reservation_id).await?.is_none());
    Ok(())
}
