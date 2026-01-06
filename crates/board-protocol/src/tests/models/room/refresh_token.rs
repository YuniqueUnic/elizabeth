use chrono::{Duration, Utc};

use crate::models::room::refresh_token::{RoomRefreshToken, TokenBlacklistEntry};

#[test]
fn token_hashing_is_deterministic() {
    let token = "test_refresh_token";
    let hash1 = RoomRefreshToken::hash_token(token);
    let hash2 = RoomRefreshToken::hash_token(token);

    assert_eq!(hash1, hash2);
    assert_ne!(token, hash1); // 确保哈希值与原令牌不同
    assert_eq!(hash1.len(), 64); // SHA-256 哈希长度
}

#[test]
fn token_verification_accepts_correct_token() {
    let token = "test_refresh_token";
    let wrong_token = "wrong_token";

    let refresh_token = RoomRefreshToken::new(
        1,
        "test_jti".to_string(),
        token,
        Utc::now().naive_utc() + Duration::hours(24),
    );

    assert!(refresh_token.verify_token(token));
    assert!(!refresh_token.verify_token(wrong_token));
}

#[test]
fn expiry_checks_work() {
    let now = Utc::now().naive_utc();

    // 未过期的令牌
    let valid_token =
        RoomRefreshToken::new(1, "test_jti".to_string(), "token", now + Duration::hours(1));
    assert!(!valid_token.is_expired());
    assert!(valid_token.is_valid());

    // 已过期的令牌
    let expired_token =
        RoomRefreshToken::new(1, "test_jti".to_string(), "token", now - Duration::hours(1));
    assert!(expired_token.is_expired());
    assert!(!expired_token.is_valid());
}

#[test]
fn token_revocation_invalidates_token() {
    let mut token = RoomRefreshToken::new(
        1,
        "test_jti".to_string(),
        "token",
        Utc::now().naive_utc() + Duration::hours(24),
    );

    assert!(!token.is_revoked);
    assert!(token.is_valid());

    token.revoke();
    assert!(token.is_revoked);
    assert!(!token.is_valid());
}

#[test]
fn last_used_update_sets_timestamp() {
    let mut token = RoomRefreshToken::new(
        1,
        "test_jti".to_string(),
        "token",
        Utc::now().naive_utc() + Duration::hours(24),
    );

    assert!(token.last_used_at.is_none());

    let before_update = Utc::now().naive_utc();
    token.update_last_used();
    let after_update = Utc::now().naive_utc();

    assert!(token.last_used_at.is_some());
    let last_used = token.last_used_at.unwrap();
    assert!(last_used >= before_update);
    assert!(last_used <= after_update);
}

#[test]
fn remaining_seconds_is_positive_for_unexpired_token() {
    let now = Utc::now().naive_utc();
    let token = RoomRefreshToken::new(1, "test_jti".to_string(), "token", now + Duration::hours(1));

    let remaining = token.remaining_seconds();
    assert!(remaining > 3500); // 约 1 小时 = 3600 秒，允许一些误差
    assert!(remaining <= 3600);
}

#[test]
fn blacklist_entry_validity_checks_work() {
    let now = Utc::now().naive_utc();
    let entry = TokenBlacklistEntry::new("test_jti".to_string(), now + Duration::hours(24));

    assert!(!entry.is_expired());
    assert!(entry.is_valid());

    let expired_entry = TokenBlacklistEntry::new("test_jti".to_string(), now - Duration::hours(1));

    assert!(expired_entry.is_expired());
    assert!(!expired_entry.is_valid());
}
