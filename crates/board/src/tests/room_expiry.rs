use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use chrono::{NaiveDate, Utc};
use tower::ServiceExt;

use crate::config::{
    AppConfig, DEFAULT_ROOM_AGE_SECONDS, DEFAULT_ROOM_ALLOWED_AGES_SECONDS, RoomExpiryPolicy,
};
use crate::db::{DbPoolSettings, init_db, run_migrations};
use crate::dto::admin::RoomExpiryOverride;
use crate::dto::rooms::{CreateRoomRequest, UpdateRoomSettingsRequest};
use crate::handlers::admin::validated_room_expiry_override;
use crate::handlers::config::get_public_config;
use crate::handlers::rooms::lifecycle::create;
use crate::handlers::rooms::settings::apply_validated_settings_payload;
use crate::handlers::rooms::shared::apply_room_defaults;
use crate::models::Room;
use crate::state::AppState;

#[test]
fn default_policy_matches_deployment_defaults() {
    let policy = RoomExpiryPolicy::default();

    assert_eq!(
        policy.allowed_ages_seconds(),
        DEFAULT_ROOM_ALLOWED_AGES_SECONDS
    );
    assert_eq!(policy.default_age_seconds(), DEFAULT_ROOM_AGE_SECONDS);
}

#[test]
fn policy_rejects_invalid_allowed_age_sets() {
    assert!(RoomExpiryPolicy::new(vec![], 60).is_err());
    assert!(RoomExpiryPolicy::new(vec![0, 60], 60).is_err());
    assert!(RoomExpiryPolicy::new(vec![60, 60], 60).is_err());
    assert!(RoomExpiryPolicy::new(vec![120, 60], 60).is_err());
    assert!(RoomExpiryPolicy::new(vec![60, 120], 90).is_err());
}

#[test]
fn policy_computes_expiry_only_for_allowed_ages() {
    let policy = RoomExpiryPolicy::new(vec![60, 120], 120).expect("valid policy");
    let now = NaiveDate::from_ymd_opt(2026, 7, 10)
        .expect("valid date")
        .and_hms_opt(12, 0, 0)
        .expect("valid time");

    assert_eq!(
        policy.expire_at(now, 60),
        now.checked_add_signed(chrono::Duration::seconds(60))
    );
    assert_eq!(policy.default_expire_at(now), policy.expire_at(now, 120));
    assert_eq!(policy.expire_at(now, 90), None);
}

#[test]
fn config_boundary_converts_human_durations_to_seconds() {
    let external = configrs::RoomExpiryConfig {
        allowed_ages: vec![
            Duration::from_secs(60).into(),
            Duration::from_secs(7200).into(),
        ],
        default_age: Duration::from_secs(7200).into(),
    };

    let policy = RoomExpiryPolicy::try_from(&external).expect("valid external config");
    assert_eq!(policy.allowed_ages_seconds(), [60, 7200]);
    assert_eq!(policy.default_age_seconds(), 7200);
}

/// 单连接 + 已跑迁移的内存库，供需要读写 rooms 表的用例复用。
async fn test_state_with_policy(policy: RoomExpiryPolicy) -> Result<Arc<AppState>> {
    let settings = DbPoolSettings::new("sqlite::memory:")
        .with_max_connections(1)
        .with_min_connections(1);
    let pool = Arc::new(init_db(&settings).await?);
    run_migrations(&pool, &settings.url).await?;
    let mut config = AppConfig::for_development();
    config.room.expiry = policy;
    Ok(Arc::new(AppState::new(config, pool)?))
}

#[tokio::test]
async fn public_config_exposes_only_normalized_room_expiry_policy() -> Result<()> {
    let app_state = test_state_with_policy(RoomExpiryPolicy::new(vec![60, 7200], 7200)?).await?;

    let response = get_public_config(State(app_state)).await.0;

    assert_eq!(response.room.expiry.allowed_ages_seconds, vec![60, 7200]);
    assert_eq!(response.room.expiry.default_age_seconds, 7200);
    Ok(())
}

#[tokio::test]
async fn public_config_route_is_unauthenticated_and_does_not_leak_private_config() -> Result<()> {
    let app_state = test_state_with_policy(RoomExpiryPolicy::new(vec![60, 7200], 7200)?).await?;
    let (router, _) = crate::route::config::api_router(app_state).split_for_parts();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(
        json["room"]["expiry"]["allowed_ages_seconds"],
        serde_json::json!([60, 7200])
    );
    assert!(json.get("jwt").is_none());
    assert!(json.get("database").is_none());
    assert!(json.get("storage").is_none());
    assert!(!String::from_utf8_lossy(&body).contains("jwt_secret"));
    Ok(())
}

#[tokio::test]
async fn room_defaults_apply_the_configured_expiry_age() -> Result<()> {
    let policy = RoomExpiryPolicy::new(vec![60, 7200], 7200)?;
    let app_state = test_state_with_policy(policy.clone()).await?;
    let mut room = Room::new("expiry-default-room".to_string(), None);

    apply_room_defaults(&mut room, &app_state, &policy)?;

    assert_eq!(
        room.expire_at,
        room.created_at
            .checked_add_signed(chrono::Duration::seconds(7200))
    );
    Ok(())
}

fn settings_request(age_seconds: i64) -> UpdateRoomSettingsRequest {
    UpdateRoomSettingsRequest {
        password: None,
        remove_password: None,
        age_seconds: Some(age_seconds),
        max_times_entered: None,
        max_size: None,
        default_role_key: None,
        upload_file_type: None,
    }
}

#[test]
fn allowed_age_updates_room_expiry_using_server_time() -> Result<()> {
    let policy = RoomExpiryPolicy::new(vec![60, 7200], 7200)?;
    let mut room = Room::new("allowed-expiry-room".to_string(), None);
    let before = Utc::now().naive_utc() + chrono::Duration::seconds(60);

    apply_validated_settings_payload(&mut room, settings_request(60), &policy)?;

    let after = Utc::now().naive_utc() + chrono::Duration::seconds(60);
    let expire_at = room.expire_at.expect("expiry applied");
    assert!(expire_at >= before);
    assert!(expire_at <= after);
    Ok(())
}

#[test]
fn disallowed_age_returns_bad_request_without_mutating_room_expiry() -> Result<()> {
    let policy = RoomExpiryPolicy::new(vec![60, 7200], 7200)?;
    let mut room = Room::new("disallowed-expiry-room".to_string(), None);
    room.expire_at = Some(Utc::now().naive_utc() + chrono::Duration::hours(1));
    let original_expire_at = room.expire_at;

    let error = apply_validated_settings_payload(&mut room, settings_request(90), &policy)
        .expect_err("disallowed age must fail");

    assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
    assert_eq!(room.expire_at, original_expire_at);
    Ok(())
}

fn create_request(age_seconds: Option<i64>) -> CreateRoomRequest {
    CreateRoomRequest {
        password: None,
        admin_identity_code: None,
        age_seconds,
    }
}

/// 落库时 created_at 取插入时刻，而 expire_at 由更早的内存时刻推算，
/// 因此寿命与标称时长存在亚秒级偏差，断言留 1 秒容差。
fn assert_lifetime(
    expire_at: Option<chrono::NaiveDateTime>,
    created_at: chrono::NaiveDateTime,
    expected_seconds: i64,
) {
    let lifetime = expire_at.expect("expiry applied") - created_at;
    let drift = (lifetime - chrono::Duration::seconds(expected_seconds))
        .num_milliseconds()
        .abs();
    assert!(
        drift <= 1_000,
        "lifetime {lifetime:?} should be within 1s of {expected_seconds}s"
    );
}

/// 建房时选定的有效期必须直接决定新房间的截止时刻，而不是永远取部署默认值。
#[tokio::test]
async fn creating_a_room_applies_the_requested_expiry_age() -> Result<()> {
    let state = test_state_with_policy(RoomExpiryPolicy::new(vec![60, 7200], 7200)?).await?;

    let axum::Json(created) = create(
        Path("chosen-age-room".to_string()),
        State(state.clone()),
        axum::Json(create_request(Some(60))),
    )
    .await?;

    let room = created.room;
    assert_lifetime(room.expire_at, room.created_at, 60);
    Ok(())
}

/// 缺省时不传 age_seconds，仍按部署默认时长建房。
#[tokio::test]
async fn creating_a_room_without_an_age_uses_the_deployment_default() -> Result<()> {
    let state = test_state_with_policy(RoomExpiryPolicy::new(vec![60, 7200], 7200)?).await?;

    let axum::Json(created) = create(
        Path("default-age-room".to_string()),
        State(state.clone()),
        axum::Json(create_request(None)),
    )
    .await?;

    let room = created.room;
    assert_lifetime(room.expire_at, room.created_at, 7200);
    Ok(())
}

/// 不在允许列表内的建房时长必须被拒，且不得留下任何房间。
#[tokio::test]
async fn creating_a_room_rejects_an_age_outside_the_allowed_set() -> Result<()> {
    let state = test_state_with_policy(RoomExpiryPolicy::new(vec![60, 7200], 7200)?).await?;

    let error = create(
        Path("rejected-age-room".to_string()),
        State(state.clone()),
        axum::Json(create_request(Some(90))),
    )
    .await
    .expect_err("age outside the allowed set must be rejected");

    assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE name = $1")
        .bind("rejected-age-room")
        .fetch_one(&*state.db_pool)
        .await?;
    assert_eq!(count, 0, "rejected creation must not persist a room");
    Ok(())
}

#[test]
fn runtime_expiry_override_validation_rejects_inconsistent_sets() {
    let cleared = validated_room_expiry_override(RoomExpiryOverride {
        allowed_ages_seconds: vec![],
        default_age_seconds: 0,
    })
    .expect("empty allowed list clears the override");
    assert!(cleared.is_none());

    let accepted = validated_room_expiry_override(RoomExpiryOverride {
        allowed_ages_seconds: vec![60, 120],
        default_age_seconds: 120,
    })
    .expect("valid override");
    assert_eq!(
        accepted.expect("override present").allowed_ages_seconds(),
        [60, 120]
    );

    for invalid in [
        // 默认时长不属于允许列表
        RoomExpiryOverride {
            allowed_ages_seconds: vec![60, 120],
            default_age_seconds: 90,
        },
        // 允许列表未严格递增
        RoomExpiryOverride {
            allowed_ages_seconds: vec![120, 60],
            default_age_seconds: 60,
        },
        // 允许时长为 0
        RoomExpiryOverride {
            allowed_ages_seconds: vec![0, 60],
            default_age_seconds: 60,
        },
    ] {
        assert!(
            validated_room_expiry_override(invalid.clone()).is_err(),
            "override {invalid:?} must be rejected"
        );
    }
}

/// 运行时覆盖必须同时驱动生效策略与公开配置；清除后回到配置文件值。
#[tokio::test]
async fn runtime_room_expiry_override_drives_the_effective_policy() -> Result<()> {
    let state = test_state_with_policy(RoomExpiryPolicy::new(vec![60, 7200], 7200)?).await?;
    assert_eq!(
        state.room_expiry_policy().allowed_ages_seconds(),
        [60, 7200]
    );

    state
        .runtime
        .set_room_expiry_policy(Some(Arc::new(RoomExpiryPolicy::new(
            vec![120, 3600],
            3600,
        )?)));

    let effective = state.room_expiry_policy();
    assert_eq!(effective.allowed_ages_seconds(), [120, 3600]);
    assert_eq!(effective.default_age_seconds(), 3600);

    let response = get_public_config(State(state.clone())).await.0;
    assert_eq!(response.room.expiry.allowed_ages_seconds, vec![120, 3600]);
    assert_eq!(response.room.expiry.default_age_seconds, 3600);

    state.runtime.set_room_expiry_policy(None);
    assert_eq!(
        state.room_expiry_policy().allowed_ages_seconds(),
        [60, 7200]
    );
    assert_eq!(state.room_expiry_policy().default_age_seconds(), 7200);
    Ok(())
}

/// 建房默认时长同样跟随运行时覆盖，而不是只看配置文件。
#[tokio::test]
async fn runtime_override_changes_the_default_age_for_new_rooms() -> Result<()> {
    let state = test_state_with_policy(RoomExpiryPolicy::new(vec![60, 7200], 7200)?).await?;
    state
        .runtime
        .set_room_expiry_policy(Some(Arc::new(RoomExpiryPolicy::new(vec![300, 900], 900)?)));

    let axum::Json(created) = create(
        Path("overridden-default-room".to_string()),
        State(state.clone()),
        axum::Json(create_request(None)),
    )
    .await?;

    let room = created.room;
    assert_lifetime(room.expire_at, room.created_at, 900);
    Ok(())
}
