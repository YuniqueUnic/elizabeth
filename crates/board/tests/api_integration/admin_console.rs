//! 平台管理后台 API（issue #196 第一阶段）：
//! 房间管理（列表/搜索/详情/删除）、dashboard 统计、存储状态、配置只读视图。
//! 鉴权沿用 `X-Elizabeth-Admin-Token` / `ELIZABETH_ADMIN_TOKEN`，未配置即整体关闭。

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use serial_test::serial;
use tower::ServiceExt;

use crate::common::create_test_app;

const ADMIN_TOKEN: &str = "admin-console-test-token";
const ADMIN_HEADER: &str = "X-Elizabeth-Admin-Token";

async fn body_json(response: axum::response::Response) -> Result<Value> {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn admin_request(method: Method, uri: &str) -> Result<Request<Body>> {
    let mut builder = Request::builder().method(method).uri(uri);
    builder = builder.header(ADMIN_HEADER, ADMIN_TOKEN);
    Ok(builder.body(Body::empty())?)
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

fn set_admin_env(value: Option<&str>) {
    // SAFETY: admin 测试经 #[serial] 串行执行，进程内无并发 env 访问。
    match value {
        Some(value) => unsafe { std::env::set_var("ELIZABETH_ADMIN_TOKEN", value) },
        None => unsafe { std::env::remove_var("ELIZABETH_ADMIN_TOKEN") },
    }
}

#[tokio::test]
#[serial]
async fn test_admin_api_disabled_without_token_config() -> Result<()> {
    set_admin_env(None);
    let (app, _pool) = create_test_app().await?;

    for uri in [
        "/api/v1/admin/stats",
        "/api/v1/admin/rooms",
        "/api/v1/admin/storage",
        "/api/v1/admin/config",
    ] {
        let response = app
            .clone()
            .oneshot(admin_request(Method::GET, uri)?)
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{uri}");
        let body = body_json(response).await?;
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Admin API disabled"),
            "{uri} 应在未配置 token 时整体关闭：{body}"
        );
    }

    // 配置了 token 之后即恢复可用
    set_admin_env(Some(ADMIN_TOKEN));
    let response = app
        .clone()
        .oneshot(admin_request(Method::GET, "/api/v1/admin/stats")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    set_admin_env(None);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_endpoints_reject_invalid_token() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/stats")
        .header(ADMIN_HEADER, "wrong-token")
        .body(Body::empty())?;
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    set_admin_env(None);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_stats_reflects_rooms_and_contents() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;
    let token = create_room(&app, "stats-room").await?;

    let payload: &'static [u8] = b"admin stats payload";
    let put = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/rooms/stats-room/files/report.txt")
        .header("content-length", payload.len().to_string())
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(payload))?;
    let response = app.clone().oneshot(put).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(admin_request(Method::GET, "/api/v1/admin/stats")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let stats = body_json(response).await?;
    assert!(stats["rooms_total"].as_i64().unwrap() >= 1);
    assert!(stats["contents_files"].as_i64().unwrap() >= 1);
    assert_eq!(stats["blob_count"], json!(1));
    assert_eq!(stats["storage_logical_bytes"], json!(payload.len() as i64));
    assert_eq!(stats["storage_physical_bytes"], json!(payload.len() as i64));
    set_admin_env(None);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_room_list_search_and_detail() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;
    create_room(&app, "findme-alpha").await?;
    create_room(&app, "unrelated-beta").await?;

    // 列表 + 搜索
    let response = app
        .clone()
        .oneshot(admin_request(Method::GET, "/api/v1/admin/rooms?q=findme")?)
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
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    set_admin_env(None);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_delete_room_purges_everything() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, pool) = create_test_app().await?;
    let token = create_room(&app, "doomed-room").await?;

    let payload: &'static [u8] = b"doomed payload";
    let put = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/rooms/doomed-room/files/doomed.txt")
        .header("content-length", payload.len().to_string())
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(payload))?;
    assert_eq!(app.clone().oneshot(put).await?.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(admin_request(
            Method::DELETE,
            "/api/v1/admin/rooms/doomed-room",
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
    set_admin_env(None);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_storage_and_config_views() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;

    let response = app
        .clone()
        .oneshot(admin_request(Method::GET, "/api/v1/admin/storage")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let storage = body_json(response).await?;
    assert_eq!(storage["backend"], json!("fs"));
    assert_eq!(storage["transfer_mode"], json!("proxy"));
    assert!(!storage["root"].as_str().unwrap().is_empty());
    assert!(storage.get("bucket").is_none() || storage["bucket"].is_null());

    let response = app
        .clone()
        .oneshot(admin_request(Method::GET, "/api/v1/admin/config")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let config = body_json(response).await?;
    assert_eq!(config["database_backend"], json!("sqlite"));
    assert_eq!(config["admin_api_enabled"], json!(true));
    let raw = config.to_string();
    assert!(
        !raw.contains("test-secret-key-for-unit-testing-123456789"),
        "配置视图不得回显机密"
    );
    set_admin_env(None);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_runtime_config_update_roundtrip() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;

    // 默认值：robots 防索引开启（安全默认）
    let response = app
        .clone()
        .oneshot(admin_request(Method::GET, "/api/v1/admin/config")?)
        .await?;
    let config = body_json(response).await?;
    assert_eq!(config["runtime_disallow_search_indexing"], json!(true));
    assert_eq!(config["dedup_scope"], json!("per-room"));
    assert_eq!(config["runtime_room_default_max_size"], json!(0));

    // 运行时关闭防索引 → robots.txt 立即允许抓取
    let update = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/admin/config/runtime")
        .header(ADMIN_HEADER, ADMIN_TOKEN)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"disallow_search_indexing": false}).to_string(),
        ))?;
    let response = app.clone().oneshot(update).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let updated = body_json(response).await?;
    assert_eq!(updated["runtime_disallow_search_indexing"], json!(false));

    // robots.txt 的动态生效由 E2E（真实服务器装配）覆盖。

    // 新房间默认容量覆盖生效（1 MiB 上限内取值）
    let update = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/admin/config/runtime")
        .header(ADMIN_HEADER, ADMIN_TOKEN)
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
    .fetch_one(_pool.as_ref())
    .await?;
    assert_eq!(max_size, 1048576);
    assert_eq!(max_times, 7);

    // 参数越界 → 400
    let update = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/admin/config/runtime")
        .header(ADMIN_HEADER, ADMIN_TOKEN)
        .header("content-type", "application/json")
        .body(Body::from(json!({"room_default_max_size": 1}).to_string()))?;
    let response = app.clone().oneshot(update).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 未授权 → 403
    let update = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/admin/config/runtime")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"disallow_search_indexing": true}).to_string(),
        ))?;
    let response = app.clone().oneshot(update).await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 还原防索引默认
    let update = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/admin/config/runtime")
        .header(ADMIN_HEADER, ADMIN_TOKEN)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"disallow_search_indexing": true}).to_string(),
        ))?;
    assert_eq!(app.clone().oneshot(update).await?.status(), StatusCode::OK);
    set_admin_env(None);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_runtime_config_extended_fields() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;

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

fn admin_request_with_body(method: Method, uri: &str, body: Value) -> Result<Request<Body>> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(ADMIN_HEADER, ADMIN_TOKEN)
        .header("content-type", "application/json");
    Ok(builder.body(Body::from(body.to_string()))?)
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

#[tokio::test]
#[serial]
async fn test_admin_credential_rotation() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;

    // 强度不足被拒绝
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/credential",
            json!({ "token": "short" }),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 轮换后旧凭证失效、新凭证生效
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/credential",
            json!({ "token": "rotated-admin-token-123" }),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await?;
    assert_eq!(body["admin_token_source"], "runtime-override");

    let old_response = app
        .clone()
        .oneshot(admin_request(Method::GET, "/api/v1/admin/stats")?)
        .await?;
    assert_eq!(old_response.status(), StatusCode::FORBIDDEN);

    let mut new_request = admin_request(Method::GET, "/api/v1/admin/config")?;
    new_request
        .headers_mut()
        .insert(ADMIN_HEADER, "rotated-admin-token-123".parse()?);
    let new_response = app.clone().oneshot(new_request).await?;
    assert_eq!(new_response.status(), StatusCode::OK);
    let config = body_json(new_response).await?;
    assert_eq!(config["admin_token_source"], "runtime-override");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_login_lockout_after_repeated_failures() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;

    for _ in 0..5 {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/stats")
            .header(ADMIN_HEADER, "definitely-wrong-token")
            .body(Body::empty())?;
        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    // 第 6 次即使凭证正确也进入锁定（按客户端计数）
    let response = app
        .clone()
        .oneshot(admin_request(Method::GET, "/api/v1/admin/stats")?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_updates_room_settings_and_mints_identity_code() -> Result<()> {
    set_admin_env(Some(ADMIN_TOKEN));
    let (app, _pool) = create_test_app().await?;
    create_room(&app, "admin-editable-room").await?;

    // 更新进入次数 + 设置房间密码
    let response = app
        .clone()
        .oneshot(admin_request_with_body(
            Method::PUT,
            "/api/v1/admin/rooms/admin-editable-room",
            json!({ "max_times_entered": 7, "password": "new-room-pass" }),
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
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
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
