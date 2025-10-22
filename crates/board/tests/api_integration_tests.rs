//! API 集成测试
//!
//! 测试 HTTP API 端点的完整功能

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use chrono::Duration;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

use board::models::room::{DEFAULT_MAX_ROOM_CONTENT_SIZE, DEFAULT_MAX_TIMES_ENTER_ROOM};
use board::repository::room_refresh_token_repository::{
    SqliteRoomRefreshTokenRepository, SqliteTokenBlacklistRepository,
};
use board::route::room::api_router;
use board::services::{RoomTokenService, refresh_token_service::RefreshTokenService};
use board::state::{AppState, RoomDefaults};

const TEST_UPLOAD_RESERVATION_TTL_SECONDS: i64 = 10;

/// 创建测试应用
async fn create_test_app() -> Result<(Router, SqlitePool)> {
    let pool = SqlitePool::connect(":memory:").await?;

    // 运行迁移
    sqlx::migrate!("./migrations").run(&pool).await?;

    let pool_arc = Arc::new(pool.clone());
    let token_service =
        RoomTokenService::with_config(Arc::new("test-secret".to_string()), 30 * 60, 5);
    let storage_root =
        std::env::temp_dir().join(format!("elizabeth-board-tests-{}", Uuid::new_v4()));

    // 创建刷新令牌服务
    let refresh_repo = Arc::new(SqliteRoomRefreshTokenRepository::new(pool_arc.clone()));
    let blacklist_repo = Arc::new(SqliteTokenBlacklistRepository::new(pool_arc.clone()));
    let refresh_service =
        RefreshTokenService::with_defaults(token_service.clone(), refresh_repo, blacklist_repo);

    let app_state = Arc::new(AppState::new(
        pool_arc,
        storage_root,
        Duration::seconds(TEST_UPLOAD_RESERVATION_TTL_SECONDS),
        RoomDefaults {
            max_size: DEFAULT_MAX_ROOM_CONTENT_SIZE,
            max_times_entered: DEFAULT_MAX_TIMES_ENTER_ROOM,
        },
        token_service,
        refresh_service,
    ));
    let app = api_router(app_state).into();

    Ok((app, pool))
}

#[tokio::test]
async fn test_create_room_api() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试创建房间
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/test_room?password=test123")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(response_json["name"], "test_room");
    assert!(response_json["id"].is_number());

    Ok(())
}

#[tokio::test]
async fn test_create_duplicate_room() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 第一次创建房间
    let request1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/duplicate_test")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let response1 = app.clone().oneshot(request1).await?;
    assert_eq!(response1.status(), StatusCode::OK);

    // 第二次创建同名房间应该失败
    let request2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/duplicate_test")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let response2 = app.clone().oneshot(request2).await?;
    assert_eq!(response2.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response2.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(
        response_json["message"]
            .as_str()
            .unwrap()
            .contains("already exists")
    );

    Ok(())
}

#[tokio::test]
async fn test_create_room_with_invalid_name() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试空房间名
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND); // 空路径导致 404

    Ok(())
}

#[tokio::test]
async fn test_find_room_api() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 先创建一个房间
    let create_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/find_test")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 查找房间
    let find_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/find_test")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let find_response = app.clone().oneshot(find_request).await?;
    assert_eq!(find_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(find_response.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(response_json["name"], "find_test");
    assert!(response_json["id"].is_number());

    Ok(())
}

#[tokio::test]
async fn test_find_nonexistent_room() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 查找不存在的房间 - 根据业务逻辑，会自动创建房间
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/nonexistent")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(response_json["name"], "nonexistent");
    assert!(response_json["id"].is_number());

    Ok(())
}

#[tokio::test]
async fn test_delete_room_api() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 先创建一个房间
    let create_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/delete_test")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 删除房间
    let delete_request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/rooms/delete_test")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let delete_response = app.clone().oneshot(delete_request).await?;
    let delete_status = delete_response.status();
    if delete_status != StatusCode::OK {
        let body = axum::body::to_bytes(delete_response.into_body(), usize::MAX).await?;
        panic!(
            "delete failed status {} body {}",
            delete_status,
            String::from_utf8_lossy(&body)
        );
    }

    let body = axum::body::to_bytes(delete_response.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(
        response_json["message"]
            .as_str()
            .unwrap()
            .contains("deleted successfully")
    );

    // 验证房间已被删除 - 根据业务逻辑，查找不存在的房间会自动创建
    let find_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/delete_test")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let find_response = app.clone().oneshot(find_request).await?;
    assert_eq!(find_response.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_delete_nonexistent_room() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 删除不存在的房间
    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/rooms/nonexistent")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(
        response_json["message"]
            .as_str()
            .unwrap()
            .contains("not found")
    );

    Ok(())
}

#[tokio::test]
async fn test_complete_crud_workflow() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "workflow_test";

    // 1. 创建房间
    let create_request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/{}?password=secret123", room_name))
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let create_json: serde_json::Value = serde_json::from_slice(&create_body)?;
    let room_id = create_json["id"].as_i64().unwrap();

    // 2. 查找房间
    let find_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/rooms/{}", room_name))
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let find_response = app.clone().oneshot(find_request).await?;
    assert_eq!(find_response.status(), StatusCode::OK);

    let find_body = axum::body::to_bytes(find_response.into_body(), usize::MAX).await?;
    let find_json: serde_json::Value = serde_json::from_slice(&find_body)?;

    assert_eq!(find_json["id"].as_i64().unwrap(), room_id);
    assert_eq!(find_json["name"].as_str().unwrap(), room_name);
    assert_eq!(find_json["password"].as_str().unwrap(), "secret123");

    // 3. 删除房间
    let delete_request = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/rooms/{}", room_name))
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let delete_response = app.clone().oneshot(delete_request).await?;
    assert_eq!(delete_response.status(), StatusCode::OK);

    // 4. 验证房间已删除 - 根据业务逻辑，查找不存在的房间会自动创建
    let verify_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/rooms/{}", room_name))
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let verify_response = app.clone().oneshot(verify_request).await?;
    assert_eq!(verify_response.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_room_token_and_content_flow() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 创建房间
    let create_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/content_room?password=secret")
        .header("content-type", "application/json")
        .body(Body::empty())?;
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 签发 token
    let issue_payload = serde_json::json!({ "password": "secret" });
    let issue_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/content_room/tokens")
        .header("content-type", "application/json")
        .body(Body::from(issue_payload.to_string()))?;
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let token_json: serde_json::Value = serde_json::from_slice(&body)?;
    let token = token_json["token"]
        .as_str()
        .expect("token string")
        .to_string();

    // 校验 token
    let validate_payload = serde_json::json!({ "token": token });
    let validate_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/content_room/tokens/validate")
        .header("content-type", "application/json")
        .body(Body::from(validate_payload.to_string()))?;
    let validate_response = app.clone().oneshot(validate_request).await?;
    assert_eq!(validate_response.status(), StatusCode::OK);

    // 上传前预检
    let prepare_payload = serde_json::json!({
        "files": [{
            "name": "hello.txt",
            "size": 11,
            "mime": "text/plain"
        }]
    });
    let prepare_request = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/rooms/content_room/contents/prepare?token={}",
            token
        ))
        .header("content-type", "application/json")
        .body(Body::from(prepare_payload.to_string()))?;
    let prepare_response = app.clone().oneshot(prepare_request).await?;
    assert_eq!(prepare_response.status(), StatusCode::OK);
    let prepare_body = axum::body::to_bytes(prepare_response.into_body(), usize::MAX).await?;
    let prepare_json: serde_json::Value = serde_json::from_slice(&prepare_body)?;
    let reservation_id = prepare_json["reservation_id"]
        .as_i64()
        .expect("reservation id");

    // 上传文件
    let boundary = "----elizabeth-test-boundary";
    let file_body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"hello.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         hello world\r\n\
         --{boundary}--\r\n"
    );
    let upload_request = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/rooms/content_room/contents?token={}&reservation_id={}",
            token, reservation_id
        ))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(file_body))?;
    let upload_response = app.clone().oneshot(upload_request).await?;
    assert_eq!(upload_response.status(), StatusCode::OK);

    // 列出文件
    let list_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/rooms/content_room/contents?token={}",
            token
        ))
        .header("content-type", "application/json")
        .body(Body::empty())?;
    let list_response = app.clone().oneshot(list_request).await?;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = axum::body::to_bytes(list_response.into_body(), usize::MAX).await?;
    let list_json: serde_json::Value = serde_json::from_slice(&list_body)?;
    let items = list_json.as_array().expect("content list");
    assert!(!items.is_empty());
    let content_id = items[0]["id"].as_i64().expect("content id");

    // 下载文件
    let download_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/rooms/content_room/contents/{}?token={}",
            content_id, token
        ))
        .body(Body::empty())?;
    let download_response = app.clone().oneshot(download_request).await?;
    let download_status = download_response.status();
    if download_status != StatusCode::OK {
        let body = axum::body::to_bytes(download_response.into_body(), usize::MAX).await?;
        panic!(
            "download failed status {} body {}",
            download_status,
            String::from_utf8_lossy(&body)
        );
    }

    // 删除文件
    let delete_payload = serde_json::json!({ "ids": [content_id] });
    let delete_request = Request::builder()
        .method(Method::DELETE)
        .uri(format!(
            "/api/v1/rooms/content_room/contents?token={}",
            token
        ))
        .header("content-type", "application/json")
        .body(Body::from(delete_payload.to_string()))?;
    let delete_response = app.clone().oneshot(delete_request).await?;
    let delete_status = delete_response.status();
    if delete_status != StatusCode::OK {
        let body = axum::body::to_bytes(delete_response.into_body(), usize::MAX).await?;
        panic!(
            "delete failed status {} body {}",
            delete_status,
            String::from_utf8_lossy(&body)
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_update_room_permissions_share_toggle() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let create_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/private_room")
        .body(Body::empty())?;
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    let issue_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/private_room/tokens")
        .header("content-type", "application/json")
        .body(Body::from("{}"))?;
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token = issue_json["token"].as_str().unwrap().to_string();

    let permission_payload = serde_json::json!({
        "edit": true,
        "share": false,
        "delete": false
    });
    let permission_request = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/rooms/private_room/permissions?token={}",
            token
        ))
        .header("content-type", "application/json")
        .body(Body::from(permission_payload.to_string()))?;
    let permission_response = app.clone().oneshot(permission_request).await?;
    assert_eq!(permission_response.status(), StatusCode::OK);
    let permission_body = axum::body::to_bytes(permission_response.into_body(), usize::MAX).await?;
    let permission_json: serde_json::Value = serde_json::from_slice(&permission_body)?;
    let new_slug = permission_json["slug"].as_str().expect("slug present");
    assert_ne!(new_slug, "private_room");

    let private_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/rooms/{new_slug}"))
        .body(Body::empty())?;
    let private_response = app.clone().oneshot(private_request).await?;
    assert_eq!(private_response.status(), StatusCode::OK);

    let original_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/private_room")
        .body(Body::empty())?;
    let original_response = app.clone().oneshot(original_request).await?;
    assert_eq!(original_response.status(), StatusCode::FORBIDDEN);

    Ok(())
}

#[tokio::test]
async fn test_token_refresh_revokes_old_token() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let create_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/refresh_room")
        .body(Body::empty())?;
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    let issue_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/refresh_room/tokens")
        .header("content-type", "application/json")
        .body(Body::from("{}"))?;
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token1 = issue_json["token"].as_str().unwrap().to_string();

    let refresh_payload = serde_json::json!({ "token": token1 });
    let refresh_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/refresh_room/tokens")
        .header("content-type", "application/json")
        .body(Body::from(refresh_payload.to_string()))?;
    let refresh_response = app.clone().oneshot(refresh_request).await?;
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body = axum::body::to_bytes(refresh_response.into_body(), usize::MAX).await?;
    let refresh_json: serde_json::Value = serde_json::from_slice(&refresh_body)?;
    let token2 = refresh_json["token"].as_str().unwrap().to_string();
    assert_ne!(token1, token2);

    let validate_new_payload = serde_json::json!({ "token": token2 });
    let validate_new_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/refresh_room/tokens/validate")
        .header("content-type", "application/json")
        .body(Body::from(validate_new_payload.to_string()))?;
    let validate_new_response = app.clone().oneshot(validate_new_request).await?;
    assert_eq!(validate_new_response.status(), StatusCode::OK);

    let validate_old_payload = serde_json::json!({ "token": token1 });
    let validate_old_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/rooms/refresh_room/tokens/validate")
        .header("content-type", "application/json")
        .body(Body::from(validate_old_payload.to_string()))?;
    let validate_old_response = app.clone().oneshot(validate_old_request).await?;
    assert_eq!(validate_old_response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}
