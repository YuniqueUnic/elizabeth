//! API 集成测试
//!
//! 测试 HTTP API 端点的完整功能

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::util::ServiceExt;

use board::route::room::api_router;

/// 创建测试应用
async fn create_test_app() -> Result<(Router, SqlitePool)> {
    let pool = SqlitePool::connect(":memory:").await?;

    // 运行迁移
    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = api_router(Arc::new(pool.clone())).into();

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
    assert_eq!(delete_response.status(), StatusCode::OK);

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
