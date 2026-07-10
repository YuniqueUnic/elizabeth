#![allow(unused_variables, unused_imports, dead_code)]

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{
    create_test_app,
    http::{assert_json, assert_status, create_request as create_http_request, send_request},
};

fn create_room_request(name: &str, password: Option<&str>) -> Request<Body> {
    let payload = match password {
        Some(password) => json!({ "password": password }),
        None => json!({}),
    };
    create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{name}"),
        Some(Body::from(payload.to_string())),
    )
}

#[tokio::test]
async fn test_create_room_api() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试创建房间
    let request = create_room_request("test_room", Some("test123"));

    let response = send_request(app, request).await?;

    assert_status(&response, StatusCode::OK);

    let response_json: serde_json::Value = assert_json(response).await?;

    assert_eq!(response_json["name"], "test_room");
    assert!(response_json["id"].is_number());

    Ok(())
}

#[tokio::test]
async fn test_create_duplicate_room() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 第一次创建房间
    let request1 = create_room_request("duplicate_test", None);

    let response1 = app.clone().oneshot(request1).await?;
    assert_eq!(response1.status(), StatusCode::OK);

    // 第二次创建同名房间应该失败
    let request2 = create_room_request("duplicate_test", None);

    let response2 = app.clone().oneshot(request2).await?;
    assert_eq!(response2.status(), StatusCode::CONFLICT);

    let body = axum::body::to_bytes(response2.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&body)?;

    // 检查响应中包含错误信息
    let message = response_json["error"]["message"]
        .as_str()
        .unwrap_or_else(|| {
            // 如果没有 error.message 字段，打印整个响应以便调试
            eprintln!("Response JSON: {}", response_json);
            "no error.message field"
        });
    assert!(message.contains("already exists"));

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
        .body(Body::from("{}"))?;

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
        .body(Body::from("{}"))?;

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
    assert_eq!(response_json["password_protected"], false);

    Ok(())
}

#[tokio::test]
async fn test_find_nonexistent_room() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 查找不存在的房间现在直接返回 404，不再自动创建
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/nonexistent")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_delete_room_api() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 先创建一个房间
    let create_request = create_room_request("delete_test", None);

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 获取具有删除权限的 token
    let token_payload = json!({});
    let token_request = create_http_request(
        Method::POST,
        "/api/v1/rooms/delete_test/tokens",
        Some(Body::from(token_payload.to_string())),
    );

    let token_response = app.clone().oneshot(token_request).await?;
    assert_eq!(token_response.status(), StatusCode::OK);
    let token_body = axum::body::to_bytes(token_response.into_body(), usize::MAX).await?;
    let token_json: serde_json::Value = serde_json::from_slice(&token_body)?;
    let token = token_json["token"].as_str().unwrap().to_string();

    // 删除房间（需要提供 token）
    let delete_request = create_http_request(
        Method::DELETE,
        &format!("/api/v1/rooms/delete_test?token={}", token),
        None,
    );

    let delete_response = app.clone().oneshot(delete_request).await?;
    assert_eq!(delete_response.status(), StatusCode::OK);

    let delete_body = axum::body::to_bytes(delete_response.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&delete_body)?;

    assert!(
        response_json["message"]
            .as_str()
            .unwrap()
            .contains("deleted successfully")
    );

    // 验证房间已被删除
    let find_request = create_http_request(Method::GET, "/api/v1/rooms/delete_test", None);

    let find_response = app.clone().oneshot(find_request).await?;
    assert_eq!(find_response.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_delete_nonexistent_room() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 尝试删除不存在的房间 - 即使使用无效 token 也应该返回房间未找到
    let delete_request = create_http_request(
        Method::DELETE,
        "/api/v1/rooms/nonexistent?token=invalid_token",
        None,
    );

    let delete_response = app.clone().oneshot(delete_request).await?;
    assert_eq!(delete_response.status(), StatusCode::NOT_FOUND);

    let delete_body = axum::body::to_bytes(delete_response.into_body(), usize::MAX).await?;
    let response_json: serde_json::Value = serde_json::from_slice(&delete_body)?;

    // 检查响应中包含错误信息
    let message = response_json["error"]["message"]
        .as_str()
        .unwrap_or_else(|| {
            // 如果没有 error.message 字段，打印整个响应以便调试
            eprintln!("Response JSON: {}", response_json);
            "no error.message field"
        });
    assert!(message.contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_complete_crud_workflow() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "workflow_test";

    // 1. 创建房间
    let create_request = create_room_request(room_name, Some("secret123"));

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let create_json: serde_json::Value = serde_json::from_slice(&create_body)?;
    let room_id = create_json["id"].as_i64().unwrap();

    // 2. 查找房间
    let find_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);

    let find_response = app.clone().oneshot(find_request).await?;
    assert_eq!(find_response.status(), StatusCode::OK);

    let find_body = axum::body::to_bytes(find_response.into_body(), usize::MAX).await?;
    let find_json: serde_json::Value = serde_json::from_slice(&find_body)?;

    assert_eq!(find_json["id"].as_i64().unwrap(), room_id);
    assert_eq!(find_json["name"].as_str().unwrap(), room_name);

    // 3. 获取具有删除权限的 token
    let token_payload = json!({ "password": "secret123" });
    let token_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(token_payload.to_string())),
    );

    let token_response = app.clone().oneshot(token_request).await?;
    assert_eq!(token_response.status(), StatusCode::OK);
    let token_body = axum::body::to_bytes(token_response.into_body(), usize::MAX).await?;
    let token_json: serde_json::Value = serde_json::from_slice(&token_body)?;
    let token = token_json["token"].as_str().unwrap().to_string();

    // 4. 删除房间（需要 token）
    let delete_request = create_http_request(
        Method::DELETE,
        &format!("/api/v1/rooms/{}?token={}", room_name, token),
        None,
    );

    let delete_response = app.clone().oneshot(delete_request).await?;
    assert_eq!(delete_response.status(), StatusCode::OK);

    // 5. 验证房间已删除
    let verify_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);

    let verify_response = app.clone().oneshot(verify_request).await?;
    assert_eq!(verify_response.status(), StatusCode::NOT_FOUND);

    Ok(())
}
