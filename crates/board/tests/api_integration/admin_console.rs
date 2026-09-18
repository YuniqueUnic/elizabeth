//! 平台管理后台 API：
//! 管理员账号登录（JWT 会话）、API key 生命周期、房间管理、统计、存储、配置视图。
//! 凭证模型：bootstrap 管理员账号（`ELIZABETH_ADMIN_PASSWORD` 首启创建一次）→
//! 登录换会话令牌，或签发 API key 供机器调用；旧 `X-Elizabeth-Admin-Token` 已删除。

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use serial_test::serial;
use tower::ServiceExt;

use crate::common::create_test_app;

const ADMIN_USERNAME: &str = "admin";
const ADMIN_PASSWORD: &str = "admin-console-test-pass";

async fn body_json(response: axum::response::Response) -> Result<Value> {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// 每个测试用独立的 X-Forwarded-For 身份，避免共享进程内 AttemptGuard 的锁定计数。
fn admin_request(method: Method, uri: &str, token: &str, client_ip: &str) -> Result<Request<Body>> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Forwarded-For", client_ip);
    Ok(builder.body(Body::empty())?)
}

fn admin_request_with_body(
    method: Method,
    uri: &str,
    body: Value,
    token: &str,
    client_ip: &str,
) -> Result<Request<Body>> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Forwarded-For", client_ip)
        .header("content-type", "application/json");
    Ok(builder.body(Body::from(body.to_string()))?)
}

fn api_key_request(
    method: Method,
    uri: &str,
    secret: &str,
    client_ip: &str,
) -> Result<Request<Body>> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Elizabeth-Admin-Key", secret)
        .header("X-Forwarded-For", client_ip);
    Ok(builder.body(Body::empty())?)
}

fn raw_request(
    method: Method,
    uri: &str,
    body: Value,
    headers: &[(&str, &str)],
) -> Result<Request<Body>> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    Ok(builder.body(Body::from(body.to_string()))?)
}

/// 在库中确保管理员账号存在（bootstrap 逻辑的服务端等价物，幂等）。
async fn seed_admin_account(pool: &board::db::DbPool) -> Result<()> {
    use board::repository::{AdminAccountRepository, IAdminAccountRepository};
    let repo = AdminAccountRepository::new(std::sync::Arc::new(pool.clone()));
    if repo.find_by_username(ADMIN_USERNAME).await?.is_none() {
        board::services::admin_auth::create_account(pool, ADMIN_USERNAME, ADMIN_PASSWORD).await?;
    }
    Ok(())
}

async fn login(
    app: &axum::Router,
    username: &str,
    password: &str,
    client_ip: &str,
) -> Result<axum::response::Response> {
    Ok(app
        .clone()
        .oneshot(raw_request(
            Method::POST,
            "/api/v1/admin/auth/login",
            json!({ "username": username, "password": password }),
            &[("X-Forwarded-For", client_ip)],
        )?)
        .await?)
}

/// seed + 登录的组合：返回可用的会话令牌。
async fn seeded_session(
    app: &axum::Router,
    pool: &board::db::DbPool,
    client_ip: &str,
) -> Result<String> {
    seed_admin_account(pool).await?;
    let response = login(app, ADMIN_USERNAME, ADMIN_PASSWORD, client_ip).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await?;
    assert_eq!(body["username"], ADMIN_USERNAME);
    assert!(body["expires_at"].as_str().is_some());
    assert!(!body["token"].as_str().unwrap().is_empty());
    Ok(body["token"].as_str().unwrap().to_string())
}

async fn create_room(app: &axum::Router, name: &str) -> Result<String> {
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/{name}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))?;
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(body_json(response).await?["token"]
        .as_str()
        .expect("admin token")
        .to_string())
}

#[tokio::test]
#[serial]
async fn test_admin_api_disabled_without_account() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let client = "10.0.1.1";

    for uri in [
        "/api/v1/admin/stats",
        "/api/v1/admin/rooms",
        "/api/v1/admin/storage",
        "/api/v1/admin/config",
    ] {
        let response = app
            .clone()
            .oneshot(admin_request(Method::GET, uri, "whatever", client)?)
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{uri}");
        let body = body_json(response).await?;
        let message = body["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("Admin API disabled") && message.contains("ELIZABETH_ADMIN_PASSWORD"),
            "{uri} 应提示 bootstrap 方式：{body}"
        );
    }
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_login_and_session_flow() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    seed_admin_account(pool.as_ref()).await?;

    // 错误密码 → 401
    let response = login(&app, ADMIN_USERNAME, "wrong-password", "10.0.2.1").await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 未知用户名 → 同样 401（不泄露账号是否存在）
    let response = login(&app, "no-such-admin", ADMIN_PASSWORD, "10.0.2.1").await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 正确登录 → 会话可用于 me 与管理端点
    let token = seeded_session(&app, pool.as_ref(), "10.0.2.1").await?;
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/auth/me",
            &token,
            "10.0.2.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await?["username"], ADMIN_USERNAME);

    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/stats",
            &token,
            "10.0.2.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    // 无效令牌 → 401
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/auth/me",
            "garbage",
            "10.0.2.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_endpoints_reject_without_credentials() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    seed_admin_account(pool.as_ref()).await?;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/stats")
        .body(Body::empty())?;
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_login_lockout_after_repeated_failures() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    seed_admin_account(pool.as_ref()).await?;
    let client = "10.0.3.1";

    for _ in 0..5 {
        let response = login(&app, ADMIN_USERNAME, "still-wrong", client).await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // 第 6 次即使密码正确也进入锁定（按客户端计数）
    let response = login(&app, ADMIN_USERNAME, ADMIN_PASSWORD, client).await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_stats_reflects_rooms_and_contents() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.4.1").await?;
    let room_token = create_room(&app, "stats-room").await?;

    let payload: &'static [u8] = b"admin stats payload";
    let put = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/rooms/stats-room/files/report.txt")
        .header("content-length", payload.len().to_string())
        .header("authorization", format!("Bearer {room_token}"))
        .body(Body::from(payload))?;
    let response = app.clone().oneshot(put).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/stats",
            &token,
            "10.0.4.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let stats = body_json(response).await?;
    assert!(stats["rooms_total"].as_i64().unwrap() >= 1);
    assert!(stats["contents_files"].as_i64().unwrap() >= 1);
    assert_eq!(stats["blob_count"], json!(1));
    assert_eq!(stats["storage_logical_bytes"], json!(payload.len() as i64));
    assert_eq!(stats["storage_physical_bytes"], json!(payload.len() as i64));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_room_list_search_and_detail() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.5.1").await?;
    create_room(&app, "findme-alpha").await?;
    create_room(&app, "unrelated-beta").await?;

    // 列表 + 搜索
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/rooms?q=findme",
            &token,
            "10.0.5.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let list = body_json(response).await?;
    assert_eq!(list["total"], json!(1));
    assert_eq!(list["rooms"][0]["name"], json!("findme-alpha"));
    assert_eq!(list["rooms"][0]["content_count"], json!(0));

    // 详情：计数与存在性
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/rooms/findme-alpha",
            &token,
            "10.0.5.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let detail = body_json(response).await?;
    assert_eq!(detail["name"], json!("findme-alpha"));
    assert!(detail["token_count"].as_i64().unwrap() >= 1);

    // 不存在 → 404
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/rooms/missing-room",
            &token,
            "10.0.5.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_delete_room_purges_everything() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.6.1").await?;
    let room_token = create_room(&app, "doomed-room").await?;

    let payload: &'static [u8] = b"doomed payload";
    let put = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/rooms/doomed-room/files/doomed.txt")
        .header("content-length", payload.len().to_string())
        .header("authorization", format!("Bearer {room_token}"))
        .body(Body::from(payload))?;
    assert_eq!(app.clone().oneshot(put).await?.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(admin_request(
            Method::DELETE,
            "/api/v1/admin/rooms/doomed-room",
            &token,
            "10.0.6.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    // 房间与内容全部清理
    let (room_count, content_count, blob_count): (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM rooms WHERE name = 'doomed-room'),
            (SELECT COUNT(*) FROM room_contents WHERE room_id NOT IN (SELECT id FROM rooms)),
            (SELECT COUNT(*) FROM content_blobs WHERE owner_room_id NOT IN (SELECT id FROM rooms))
        "#,
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(room_count, 0);
    assert_eq!(content_count, 0, "删除房间不得留下悬空内容");
    assert_eq!(blob_count, 0, "删除房间不得留下悬空 blob");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_storage_and_config_views() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.7.1").await?;

    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/storage",
            &token,
            "10.0.7.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let storage = body_json(response).await?;
    assert_eq!(storage["backend"], json!("fs"));
    assert_eq!(storage["transfer_mode"], json!("proxy"));
    assert!(!storage["root"].as_str().unwrap().is_empty());
    assert!(storage.get("bucket").is_none() || storage["bucket"].is_null());

    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/config",
            &token,
            "10.0.7.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let config = body_json(response).await?;
    assert_eq!(config["database_backend"], json!("sqlite"));
    assert_eq!(config["admin_api_enabled"], json!(true));
    assert!(
        config.get("admin_token_source").is_none(),
        "admin_token_source 已随 admin_token 一并删除"
    );
    let raw = config.to_string();
    assert!(
        !raw.contains(ADMIN_PASSWORD)
            && !raw.contains("test-secret-key-for-unit-testing-123456789"),
        "配置视图不得回显机密"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_runtime_config_update_roundtrip() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.8.1").await?;

    // 默认值：robots 防索引开启（安全默认）
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/config",
            &token,
            "10.0.8.1",
        )?)
        .await?;
    let config = body_json(response).await?;
    assert_eq!(config["runtime_disallow_search_indexing"], json!(true));
    assert_eq!(config["dedup_scope"], json!("per-room"));
    assert_eq!(config["runtime_room_default_max_size"], json!(0));

    // 未知字段 → 400（配置写入不得静默忽略拼写错误）
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/config/runtime",
            json!({"runtime_room_default_max_times_entered": 9}),
            &token,
            "10.0.8.1",
        )?)
        .await?;
    // 未知字段拒绝写入：axum 的 Json 提取默认以 422 拒绝反序列化失败
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 运行时关闭防索引 → 响应立即回显
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/config/runtime",
            json!({"disallow_search_indexing": false}),
            &token,
            "10.0.8.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let updated = body_json(response).await?;
    assert_eq!(updated["runtime_disallow_search_indexing"], json!(false));

    // 新房间默认容量覆盖生效（1 MiB 上限内取值）
    let update = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/admin/config/runtime")
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Forwarded-For", "10.0.8.1")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"room_default_max_size": 1048576, "room_default_max_times_entered": 7})
                .to_string(),
        ))?;
    let response = app.clone().oneshot(update).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let create = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/override-room")
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))?;
    assert_eq!(app.clone().oneshot(create).await?.status(), StatusCode::OK);
    let (max_size, max_times): (i64, i64) = sqlx::query_as(
        "SELECT max_size, max_times_entered FROM rooms WHERE name = 'override-room'",
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(max_size, 1048576);
    assert_eq!(max_times, 7);

    // 参数越界 → 400
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/config/runtime",
            json!({"room_default_max_size": 1}),
            &token,
            "10.0.8.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 未授权 → 401
    let update = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/admin/config/runtime")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"disallow_search_indexing": true}).to_string(),
        ))?;
    let response = app.clone().oneshot(update).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 还原防索引默认
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/config/runtime",
            json!({"disallow_search_indexing": true}),
            &token,
            "10.0.8.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_runtime_config_extended_fields() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.9.1").await?;
    let ip = "10.0.9.1";

    // 会话有效期 / 预留有效期 / 默认角色：合法覆盖
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/config/runtime",
            json!({
                "session_ttl_seconds": 3600,
                "upload_reservation_ttl_seconds": 600,
                "room_default_role_key": "editor"
            }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let config = body_json(response).await?;
    assert_eq!(config["runtime_session_ttl_seconds"], 3600);
    assert_eq!(config["runtime_upload_reservation_ttl_seconds"], 600);
    assert_eq!(config["runtime_room_default_role_key"], "editor");

    // 越界值被拒绝
    for (field, value) in [
        ("session_ttl_seconds", json!(1)),
        ("session_ttl_seconds", json!(999_999_999)),
        ("upload_reservation_ttl_seconds", json!(1)),
        ("room_default_role_key", json!("superuser")),
    ] {
        let response = app
            .clone()
            .oneshot(admin_request_with_body(
                Method::PUT,
                "/api/v1/admin/config/runtime",
                json!({ field: value }),
                &token,
                ip,
            )?)
            .await?;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "{field}={value}"
        );
    }

    // 传 0 / 空串清除覆盖
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/config/runtime",
            json!({
                "session_ttl_seconds": 0,
                "upload_reservation_ttl_seconds": 0,
                "room_default_role_key": ""
            }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let config = body_json(response).await?;
    assert_eq!(config["runtime_session_ttl_seconds"], 0);
    assert_eq!(config["runtime_upload_reservation_ttl_seconds"], 0);
    assert!(config["runtime_room_default_role_key"].is_null());

    // 会话有效期覆盖影响新签发的访问令牌
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/config/runtime",
            json!({ "session_ttl_seconds": 3600 }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    // 建房令牌跟随房间生命周期；会话有效期作用于新建会话（issue_token 默认 TTL）
    let _ = create_room(&app, "runtime-ttl-room").await?;
    let issue = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/runtime-ttl-room/tokens")
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))?;
    let issue_response = app.clone().oneshot(issue).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let room_token = body_json(issue_response).await?["token"]
        .as_str()
        .expect("issued token")
        .to_string();
    let claims = decode_jwt_claims(&room_token);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    let exp = claims["exp"].as_i64().expect("exp claim");
    assert!(
        (3500..=3600).contains(&(exp - now)),
        "expected ~3600s session ttl, delta = {}",
        exp - now
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_password_change_invalidates_sessions() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    seed_admin_account(pool.as_ref()).await?;
    let ip = "10.0.10.1";

    // 登录拿到两个会话
    let make_session = || async {
        let response = login(&app, ADMIN_USERNAME, ADMIN_PASSWORD, ip).await?;
        assert_eq!(response.status(), StatusCode::OK);
        Ok::<String, anyhow::Error>(
            body_json(response).await?["token"]
                .as_str()
                .unwrap()
                .to_string(),
        )
    };
    let session_a = make_session().await?;
    let session_b = make_session().await?;

    // 当前密码错误 → 401
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/auth/password",
            json!({ "current_password": "wrong-current", "new_password": "brand-new-password-1" }),
            &session_a,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 新密码强度不足 → 400
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/auth/password",
            json!({ "current_password": ADMIN_PASSWORD, "new_password": "short" }),
            &session_a,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 正确改密 → 成功
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/auth/password",
            json!({ "current_password": ADMIN_PASSWORD, "new_password": "brand-new-password-1" }),
            &session_a,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    // 改密后所有既有会话立即失效（包括发起改密的会话 A）
    for token in [&session_a, &session_b] {
        let response = app
            .clone()
            .oneshot(admin_request(
                Method::GET,
                "/api/v1/admin/auth/me",
                token,
                ip,
            )?)
            .await?;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "old session must die"
        );
    }

    // 新密码可登录，旧密码不可
    let response = login(&app, ADMIN_USERNAME, ADMIN_PASSWORD, ip).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = login(&app, ADMIN_USERNAME, "brand-new-password-1", ip).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_api_key_lifecycle() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.11.1").await?;
    let ip = "10.0.11.1";
    let key_ip = "10.0.11.2";

    // 未授权创建 → 401
    let response = app
        .clone()
        .oneshot(raw_request(
            Method::POST,
            "/api/v1/admin/api-keys",
            json!({ "name": "ci" }),
            &[],
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 创建：明文仅此一次返回
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::POST,
            "/api/v1/admin/api-keys",
            json!({ "name": "ci-key", "expires_in_secs": 3600 }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let created = body_json(response).await?;
    let secret = created["secret"]
        .as_str()
        .expect("one-time secret")
        .to_string();
    assert!(secret.starts_with("elizabeth_ak_"));
    assert_eq!(created["name"], "ci-key");
    assert!(created["expires_at"].as_str().is_some());
    let key_id = created["id"].as_i64().expect("key id");

    // 非法参数
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::POST,
            "/api/v1/admin/api-keys",
            json!({ "name": "  " }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::POST,
            "/api/v1/admin/api-keys",
            json!({ "name": "bad-ttl", "expires_in_secs": 0 }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // API key 可访问管理端点
    let response = app
        .clone()
        .oneshot(api_key_request(
            Method::GET,
            "/api/v1/admin/stats",
            &secret,
            key_ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    // 列表可见（无明文）
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::GET,
            "/api/v1/admin/api-keys",
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let keys = body_json(response).await?;
    assert_eq!(keys.as_array().unwrap().len(), 1);
    assert_eq!(keys[0]["prefix"].as_str().unwrap().len(), 8);
    assert!(keys[0].get("secret").is_none());
    let raw = keys.to_string();
    assert!(!raw.contains(&secret), "列表不得回显明文 key");

    // 吊销后立即失效；重复吊销 → 404
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::DELETE,
            &format!("/api/v1/admin/api-keys/{key_id}"),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .clone()
        .oneshot(api_key_request(
            Method::GET,
            "/api/v1/admin/stats",
            &secret,
            key_ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .clone()
        .oneshot(admin_request(
            Method::DELETE,
            &format!("/api/v1/admin/api-keys/{key_id}"),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_api_key_actas_bootstrap_token_issue() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.12.1").await?;
    let ip = "10.0.12.2";
    create_room(&app, "bootstrap-room").await?;

    // 用会话创建 API key，再用 key 引导签发非默认角色（管理补签通道）
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::POST,
            "/api/v1/admin/api-keys",
            json!({ "name": "ops" }),
            &token,
            "10.0.12.1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let secret = body_json(response).await?["secret"]
        .as_str()
        .unwrap()
        .to_string();

    let response = app
        .clone()
        .oneshot(raw_request(
            Method::POST,
            "/api/v1/rooms/bootstrap-room/tokens",
            json!({ "role": "editor" }),
            &[
                ("X-Elizabeth-Admin-Key", secret.as_str()),
                ("X-Forwarded-For", ip),
            ],
        )?)
        .await?;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "API key 应可引导补签：{}",
        body_json(response).await?
    );
    let issued = body_json(response).await?;
    assert_eq!(issued["claims"]["role"], "editor");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_updates_room_settings_and_mints_identity_code() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = seeded_session(&app, pool.as_ref(), "10.0.13.1").await?;
    let ip = "10.0.13.1";
    create_room(&app, "admin-editable-room").await?;

    // 更新进入次数 + 设置房间密码
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/rooms/admin-editable-room",
            json!({ "max_times_entered": 7, "password": "new-room-pass" }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let detail = body_json(response).await?;
    assert_eq!(detail["max_times_entered"], 7);
    assert_eq!(detail["password_protected"], true);

    // 铸造 editor 身份码：明文仅此一次返回，且可成功兑换
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::POST,
            "/api/v1/admin/rooms/admin-editable-room/identity-codes",
            json!({ "code": "", "role": "editor" }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let minted = body_json(response).await?;
    // 响应按 serde(flatten) 平铺
    assert_eq!(minted["role"], "editor");
    let code = minted["code"].as_str().expect("minted code").to_string();
    assert!(!code.is_empty());

    let redeemed = redeem_code(&app, "admin-editable-room", &code).await?;
    assert_eq!(
        redeemed.status(),
        StatusCode::OK,
        "redeem failed: {}",
        body_json(redeemed).await?
    );

    // admin 角色码跟随房间生命周期，不接受自定义时长
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::POST,
            "/api/v1/admin/rooms/admin-editable-room/identity-codes",
            json!({ "code": "", "role": "admin", "expires_in_secs": 600 }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 未知房间 → 404
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/rooms/no-such-room",
            json!({ "max_times_entered": 7 }),
            &token,
            ip,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

fn decode_jwt_claims(token: &str) -> Value {
    let payload = token.split('.').nth(1).expect("jwt payload");
    let bytes = decode_base64url(payload).expect("valid base64url payload");
    serde_json::from_slice(&bytes).expect("jwt claims json")
}

/// 无依赖的 base64url（无 padding）解码，仅供测试断言 JWT claims。
fn decode_base64url(input: &str) -> Result<Vec<u8>, &'static str> {
    fn value_of(byte: u8) -> Result<u32, &'static str> {
        match byte {
            b'A'..=b'Z' => Ok((byte - b'A') as u32),
            b'a'..=b'z' => Ok((byte - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((byte - b'0' + 52) as u32),
            b'-' => Ok(62),
            b'_' => Ok(63),
            _ => Err("invalid base64url character"),
        }
    }
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in input.bytes() {
        buffer = (buffer << 6) | value_of(byte)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    Ok(output)
}

async fn redeem_code(
    app: &axum::Router,
    room_name: &str,
    code: &str,
) -> Result<axum::response::Response> {
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/{room_name}/identity-codes/redeem"))
        .header("content-type", "application/json")
        .body(Body::from(json!({ "code": code }).to_string()))?;
    Ok(app.clone().oneshot(request).await?)
}
