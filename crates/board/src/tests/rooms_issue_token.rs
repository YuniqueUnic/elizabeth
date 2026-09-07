use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
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
        role: None,
        expires_in_secs: None,
    };
    let Json(resp) = issue_token(
        Path(room_slug.to_string()),
        HeaderMap::new(),
        State(app_state),
        Json(payload),
    )
    .await?;
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
        role: None,
        expires_in_secs: None,
    };
    let Json(resp) = issue_token(
        Path(room_slug.to_string()),
        HeaderMap::new(),
        State(app_state),
        Json(payload),
    )
    .await?;
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
        .sequence_number(0)
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

async fn mint_persisted_admin_token(
    app_state: Arc<AppState>,
    room: &Room,
) -> anyhow::Result<String> {
    let (token, claims) = app_state.token_service().issue(room, "admin")?;
    let record = crate::models::RoomToken::new(
        claims.room_id,
        claims.jti.clone(),
        "admin",
        claims.expires_at(),
    );
    crate::repository::RoomAccessRepository::new(app_state.db_pool.clone())
        .grant_new_session(claims.room_id, &record, None, Utc::now().naive_utc())
        .await?;
    Ok(token)
}

async fn create_room_with_expiry(
    app_state: Arc<AppState>,
    slug: &str,
    expire_in: chrono::Duration,
) -> anyhow::Result<Room> {
    let mut room = Room::new(slug.to_string(), None);
    room.expire_at = Some((Utc::now() + expire_in).naive_utc());
    let repo = RoomRepository::new(app_state.db_pool.clone());
    let created = repo
        .create_if_absent(&room)
        .await?
        .expect("fresh room must be created");
    Ok(created)
}

#[tokio::test]
async fn admin_identity_code_follows_room_lifetime_and_ignores_expires_in() -> anyhow::Result<()> {
    let app_state = setup_state().await?;
    let room = create_room_with_expiry(
        app_state.clone(),
        "room-admin-lifetime",
        chrono::Duration::days(30),
    )
    .await?;

    // 创建者 admin 码本身也应跟随房间生命周期
    let room_expire = room
        .expire_at
        .expect("room expiry set")
        .and_utc()
        .timestamp();
    let (_creator_token, creator_claims) = app_state.token_service().issue_with_ttl(
        &room,
        "admin",
        crate::services::token::room_lifetime_ttl(),
    )?;
    assert!(
        (creator_claims.exp - room_expire).abs() <= 6,
        "creator admin exp {} should track room expiry {}",
        creator_claims.exp,
        room_expire
    );

    // 通过签发接口轮换/补发 admin 码：携带 expires_in_secs 也必须被忽略
    let creator_token = mint_persisted_admin_token(app_state.clone(), &room).await?;
    let Json(resp) = issue_token(
        Path(room.slug.clone()),
        HeaderMap::new(),
        State(app_state.clone()),
        Json(IssueTokenRequest {
            password: None,
            token: Some(creator_token),
            with_refresh_token: false,
            role: Some("admin".to_string()),
            expires_in_secs: Some(3600),
        }),
    )
    .await?;
    let admin_expire = resp.expires_at.and_utc().timestamp();
    assert!(
        (admin_expire - room_expire).abs() <= 6,
        "admin identity code exp {} should track room expiry {}",
        admin_expire,
        room_expire
    );
    Ok(())
}

#[tokio::test]
async fn editor_identity_code_honors_configured_ttl() -> anyhow::Result<()> {
    let app_state = setup_state().await?;
    let room = create_room_with_expiry(
        app_state.clone(),
        "room-editor-ttl",
        chrono::Duration::days(30),
    )
    .await?;
    let admin_token = mint_persisted_admin_token(app_state.clone(), &room).await?;

    let before = Utc::now().timestamp();
    let Json(resp) = issue_token(
        Path(room.slug.clone()),
        HeaderMap::new(),
        State(app_state.clone()),
        Json(IssueTokenRequest {
            password: None,
            token: Some(admin_token),
            with_refresh_token: false,
            role: Some("editor".to_string()),
            expires_in_secs: Some(3600),
        }),
    )
    .await?;
    let editor_expire = resp.expires_at.and_utc().timestamp();
    assert!(
        (editor_expire - (before + 3600)).abs() <= 10,
        "editor exp {} should be ~now+3600s",
        editor_expire
    );

    // 缺省时长 = 部署配置的默认 TTL（开发配置 120 分钟）
    let admin_token = mint_persisted_admin_token(app_state.clone(), &room).await?;
    let before = Utc::now().timestamp();
    let Json(resp) = issue_token(
        Path(room.slug.clone()),
        HeaderMap::new(),
        State(app_state),
        Json(IssueTokenRequest {
            password: None,
            token: Some(admin_token),
            with_refresh_token: false,
            role: Some("editor".to_string()),
            expires_in_secs: None,
        }),
    )
    .await?;
    let default_expire = resp.expires_at.and_utc().timestamp();
    assert!(
        (default_expire - (before + 120 * 60)).abs() <= 15,
        "editor default exp {} should be ~now+120min",
        default_expire
    );
    Ok(())
}

#[tokio::test]
async fn editor_identity_code_rejects_out_of_range_ttl() -> anyhow::Result<()> {
    let app_state = setup_state().await?;
    let room = create_room_with_expiry(
        app_state.clone(),
        "room-editor-ttl-range",
        chrono::Duration::days(30),
    )
    .await?;
    let admin_token = mint_persisted_admin_token(app_state.clone(), &room).await?;

    for invalid in [
        30_i64,
        0,
        -1,
        crate::services::token::MAX_IDENTITY_TTL_SECONDS + 1,
    ] {
        let result = issue_token(
            Path(room.slug.clone()),
            HeaderMap::new(),
            State(app_state.clone()),
            Json(IssueTokenRequest {
                password: None,
                token: Some(admin_token.clone()),
                with_refresh_token: false,
                role: Some("editor".to_string()),
                expires_in_secs: Some(invalid),
            }),
        )
        .await;
        let err = result.expect_err("out-of-range ttl must be rejected");
        assert_eq!(err.status_code(), axum::http::StatusCode::BAD_REQUEST);
    }
    Ok(())
}
