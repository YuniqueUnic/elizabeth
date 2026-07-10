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
async fn test_create_room_normalizes_explicit_empty_password_to_unprotected() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let response = app
        .oneshot(create_room_request("empty-password", Some("")))
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let room: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(room["password_protected"], false);

    let stored_password: Option<String> =
        sqlx::query_scalar("SELECT password FROM rooms WHERE name = $1")
            .bind("empty-password")
            .fetch_one(pool.as_ref())
            .await?;
    assert_eq!(stored_password, None);
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
    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let created: serde_json::Value = serde_json::from_slice(&create_body)?;
    // The explicit creation flow and direct-URL provisioning must share the
    // same deployment-owned room defaults rather than maintaining two policies.
    assert_eq!(created["password_protected"], false);
    assert_eq!(created["max_size"], 50 * 1024 * 1024);
    assert_eq!(created["max_times_entered"], 100);
    assert_eq!(created["permission"], 15);

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
    let (app, pool) = create_test_app().await?;

    // Product contract: a valid missing room URL is a zero-step room creation flow.
    // Do not change this back to 404 or pre-create the room in this test: both the
    // browser direct-link UX and the deployment-defined creation defaults depend on it.
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/nonexistent")
        .header("content-type", "application/json")
        .body(Body::empty())?;

    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let room: serde_json::Value = serde_json::from_slice(&body)?;
    let room_id = room["id"].as_i64().expect("room id");

    assert_eq!(room["name"], "nonexistent");
    assert_eq!(room["slug"], "nonexistent");
    assert_eq!(room["password_protected"], false);
    assert_eq!(room["max_size"], 50 * 1024 * 1024);
    assert_eq!(room["max_times_entered"], 100);
    assert_eq!(room["permission"], 15);

    let created_at = room["created_at"]
        .as_str()
        .expect("created_at")
        .parse::<chrono::NaiveDateTime>()?;
    let expire_at = room["expire_at"]
        .as_str()
        .expect("expire_at")
        .parse::<chrono::NaiveDateTime>()?;
    let lifetime = expire_at.signed_duration_since(created_at).num_seconds();
    assert!((7199..=7200).contains(&lifetime));

    let second_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/rooms/nonexistent")
        .body(Body::empty())?;
    let second_response = app.oneshot(second_request).await?;
    assert_eq!(second_response.status(), StatusCode::OK);
    let second_body = axum::body::to_bytes(second_response.into_body(), usize::MAX).await?;
    let second_room: serde_json::Value = serde_json::from_slice(&second_body)?;
    assert_eq!(second_room["id"], room_id);

    let room_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE name = $1")
        .bind("nonexistent")
        .fetch_one(pool.as_ref())
        .await?;
    assert_eq!(room_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_find_missing_long_identifier_does_not_bypass_creation_name_rules() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let name = "a".repeat(51);
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/rooms/{name}"))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let room_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms")
        .fetch_one(pool.as_ref())
        .await?;
    assert_eq!(room_count, 0);
    Ok(())
}

#[tokio::test]
async fn test_concurrent_direct_url_requests_converge_on_one_room() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let request = || {
        Request::builder()
            .method(Method::GET)
            .uri("/api/v1/rooms/concurrent-direct")
            .body(Body::empty())
            .expect("request")
    };

    let (left, right) = tokio::join!(
        app.clone().oneshot(request()),
        app.clone().oneshot(request())
    );
    let left = left?;
    let right = right?;
    assert_eq!(left.status(), StatusCode::OK);
    assert_eq!(right.status(), StatusCode::OK);

    let left: serde_json::Value =
        serde_json::from_slice(&axum::body::to_bytes(left.into_body(), usize::MAX).await?)?;
    let right: serde_json::Value =
        serde_json::from_slice(&axum::body::to_bytes(right.into_body(), usize::MAX).await?)?;
    assert_eq!(left["id"], right["id"]);

    let room_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE name = $1")
        .bind("concurrent-direct")
        .fetch_one(pool.as_ref())
        .await?;
    assert_eq!(room_count, 1);
    Ok(())
}

#[tokio::test]
async fn test_delete_room_api() -> Result<()> {
    let (app, pool) = create_test_app().await?;

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

    // 先直接查询存储，避免 GET 的自动创建产品契约掩盖删除结果。
    let count_after_delete: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE name = $1")
        .bind("delete_test")
        .fetch_one(pool.as_ref())
        .await?;
    assert_eq!(count_after_delete, 0);

    // 删除后再次直接访问属于真实 miss，因此会创建一个新的默认房间。
    let find_request = create_http_request(Method::GET, "/api/v1/rooms/delete_test", None);

    let find_response = app.clone().oneshot(find_request).await?;
    assert_eq!(find_response.status(), StatusCode::OK);

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
    let (app, pool) = create_test_app().await?;

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

    // 5. 直接验证持久层已删除；GET miss 会按产品契约重新创建。
    let count_after_delete: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE name = $1")
        .bind(room_name)
        .fetch_one(pool.as_ref())
        .await?;
    assert_eq!(count_after_delete, 0);

    let verify_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);

    let verify_response = app.clone().oneshot(verify_request).await?;
    assert_eq!(verify_response.status(), StatusCode::OK);
    let verify_body = axum::body::to_bytes(verify_response.into_body(), usize::MAX).await?;
    let recreated: serde_json::Value = serde_json::from_slice(&verify_body)?;
    assert_ne!(recreated["id"].as_i64(), Some(room_id));
    assert_eq!(recreated["password_protected"], false);

    Ok(())
}
