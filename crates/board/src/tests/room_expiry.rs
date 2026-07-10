use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use chrono::{NaiveDate, Utc};
use tower::ServiceExt;

use crate::config::{
    AppConfig, DEFAULT_ROOM_AGE_SECONDS, DEFAULT_ROOM_ALLOWED_AGES_SECONDS, RoomExpiryPolicy,
};
use crate::db::{DbPoolSettings, init_db};
use crate::dto::rooms::UpdateRoomSettingsRequest;
use crate::handlers::config::get_public_config;
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

async fn test_state_with_policy(policy: RoomExpiryPolicy) -> Result<Arc<AppState>> {
    let pool = Arc::new(init_db(&DbPoolSettings::new("sqlite::memory:")).await?);
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
    let app_state = test_state_with_policy(RoomExpiryPolicy::new(vec![60, 7200], 7200)?).await?;
    let mut room = Room::new("expiry-default-room".to_string(), None);

    apply_room_defaults(&mut room, &app_state)?;

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
        age_seconds: Some(age_seconds),
        max_times_entered: None,
        max_size: None,
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
