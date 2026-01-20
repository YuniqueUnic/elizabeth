use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use chrono::Utc;

use crate::config::{AppConfig, AuthConfig};
use crate::db::{DbPoolSettings, init_db, run_migrations};
use crate::dto::rooms::IssueTokenRequest;
use crate::handlers::rooms::issue_token;
use crate::models::Room;
use crate::models::content::{ContentType, RoomContent};
use crate::repository::{
    IRoomContentRepository, IRoomRepository, RoomContentRepository, RoomRepository,
};
use crate::state::AppState;

async fn setup_state() -> anyhow::Result<Arc<AppState>> {
    let db_settings = DbPoolSettings::new("sqlite::memory:")
        .with_max_connections(1)
        .with_min_connections(1);
    let db_pool = Arc::new(init_db(&db_settings).await?);
    run_migrations(&db_pool, &db_settings.url).await?;

    let mut cfg = AppConfig::for_development();
    cfg.auth = AuthConfig::new("test-secret-key-for-unit-testing-123".to_string())?;

    Ok(Arc::new(AppState::new(cfg, db_pool)?))
}

async fn issue_new_token(
    app_state: Arc<AppState>,
    room_slug: &str,
) -> Result<String, crate::errors::AppError> {
    let payload = IssueTokenRequest {
        password: None,
        token: None,
        with_refresh_token: false,
    };
    let Json(resp) =
        issue_token(Path(room_slug.to_string()), State(app_state), Json(payload)).await?;
    Ok(resp.token)
}

async fn refresh_token(
    app_state: Arc<AppState>,
    room_slug: &str,
    previous_token: String,
) -> Result<String, crate::errors::AppError> {
    let payload = IssueTokenRequest {
        password: None,
        token: Some(previous_token),
        with_refresh_token: false,
    };
    let Json(resp) =
        issue_token(Path(room_slug.to_string()), State(app_state), Json(payload)).await?;
    Ok(resp.token)
}

#[tokio::test]
async fn issue_token_does_not_clear_content_when_reaching_max_entries() -> anyhow::Result<()> {
    let app_state = setup_state().await?;

    let mut room = Room::new("room-max-entries".to_string(), None);
    room.max_times_entered = 3;
    room.current_times_entered = 0;

    let room_repo = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repo.create(&room).await?;
    let room_id = room.id.expect("room id should be set");

    let content_repo = RoomContentRepository::new(app_state.db_pool.clone());
    let now = Utc::now().naive_utc();
    let mut message = RoomContent::builder()
        .room_id(room_id)
        .content_type(ContentType::Text)
        .now(now)
        .build();
    message.set_text("hello".to_string());
    content_repo.create(&message).await?;

    // 1st / 2nd / 3rd entries should succeed and must not clear content.
    let _t1 = issue_new_token(app_state.clone(), &room.slug).await?;
    let _t2 = issue_new_token(app_state.clone(), &room.slug).await?;
    let _t3 = issue_new_token(app_state.clone(), &room.slug).await?;

    let updated = room_repo
        .find_by_name(&room.slug)
        .await?
        .expect("room should exist");
    assert_eq!(updated.current_times_entered, 3);

    let contents = content_repo.list_by_room(room_id).await?;
    assert_eq!(contents.len(), 1);

    // 4th entry should be rejected (room is full).
    let err = issue_new_token(app_state.clone(), &room.slug)
        .await
        .expect_err("expected full room to reject new entry");
    assert_eq!(err.status_code(), axum::http::StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn issue_token_allows_refresh_when_room_full() -> anyhow::Result<()> {
    let app_state = setup_state().await?;

    let mut room = Room::new("room-refresh-full".to_string(), None);
    room.max_times_entered = 1;
    room.current_times_entered = 0;

    let room_repo = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repo.create(&room).await?;

    let token = issue_new_token(app_state.clone(), &room.slug).await?;
    let updated = room_repo
        .find_by_name(&room.slug)
        .await?
        .expect("room should exist");
    assert_eq!(updated.current_times_entered, 1);

    // Refresh should be allowed even when current_times_entered == max_times_entered.
    let _refreshed = refresh_token(app_state.clone(), &room.slug, token).await?;
    let updated = room_repo
        .find_by_name(&room.slug)
        .await?
        .expect("room should exist");
    assert_eq!(updated.current_times_entered, 1);

    // New entry without previous token remains rejected.
    let err = issue_new_token(app_state.clone(), &room.slug)
        .await
        .expect_err("expected full room to reject new entry");
    assert_eq!(err.status_code(), axum::http::StatusCode::UNAUTHORIZED);

    Ok(())
}
