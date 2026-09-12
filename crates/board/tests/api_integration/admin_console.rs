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
            (SELECT COUNT(*) FROM room_content_blobs WHERE room_id NOT IN (SELECT id FROM rooms))
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
