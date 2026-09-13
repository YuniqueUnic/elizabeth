use crate::services::{AttemptGuard, GuardScope};

#[test]
fn guard_allows_up_to_max_failures_then_locks() {
    let guard = AttemptGuard::new();
    let scope = GuardScope::AccessCode(100);
    let client = "test_client_1";

    assert!(guard.check(scope, client).is_ok());

    // 前 4 次失败不锁定
    for i in 1..=4 {
        let (count, locked) = guard.record_failure(scope, client);
        assert_eq!(count, i);
        assert_eq!(locked, None);
        assert!(guard.check(scope, client).is_ok());
    }

    // 第 5 次触发 60s 锁定
    let (count, locked) = guard.record_failure(scope, client);
    assert_eq!(count, 5);
    assert_eq!(locked, Some(60));

    let check = guard.check(scope, client);
    assert!(check.is_err());
    assert_eq!(
        check.unwrap_err().status_code(),
        axum::http::StatusCode::TOO_MANY_REQUESTS
    );

    // 成功清除记录
    guard.record_success(scope, client);
    assert!(guard.check(scope, client).is_ok());
}

#[test]
fn guard_scopes_and_clients_are_isolated() {
    let guard = AttemptGuard::new();

    // 锁定 AccessCode(1) 的 client_a
    for _ in 0..5 {
        guard.record_failure(GuardScope::AccessCode(1), "client_a");
    }
    assert!(guard.check(GuardScope::AccessCode(1), "client_a").is_err());

    // 同客户端不同内容、同内容不同客户端均不受影响
    assert!(guard.check(GuardScope::AccessCode(2), "client_a").is_ok());
    assert!(guard.check(GuardScope::AccessCode(1), "client_b").is_ok());

    // 不同作用域即使键相同也互相隔离
    assert!(guard.check(GuardScope::AdminLogin, "client_a").is_ok());
    assert!(guard.check(GuardScope::IdentityCode(1), "client_a").is_ok());
    assert!(guard.check(GuardScope::RoomPassword(1), "client_a").is_ok());
}

#[test]
fn guard_lockout_escalates_with_failure_count() {
    let guard = AttemptGuard::new();
    let scope = GuardScope::AdminLogin;
    let client = "escalating_client";

    // 锁定不重置计数：跨锁定窗口持续失败按次数升级（5→60s、10→300s、15→1800s）；
    // 已处于锁定时每次失败都会刷新锁定时长并再次返回。
    let mut lockouts = Vec::new();
    for i in 1..=15 {
        let (_, locked) = guard.record_failure(scope, client);
        if let Some(secs) = locked {
            lockouts.push((i, secs));
        }
    }
    assert_eq!(lockouts.first(), Some(&(5, 60)));
    assert!(lockouts.contains(&(10, 300)));
    assert!(lockouts.contains(&(15, 1800)));
    assert_eq!(lockouts.last().unwrap().1, 1800);
}
