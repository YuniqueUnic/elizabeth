//! API 集成测试
//!
//! 测试 HTTP API 端点的完整功能

mod common;

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

// 导入测试公共模块
use common::{
    create_test_app,
    fixtures::{passwords, room_names},
    http::{assert_json, assert_status, create_request as create_http_request, send_request},
};

use board::route::room::api_router;

#[tokio::test]
async fn test_create_room_api() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试创建房间
    let request = create_http_request(
        Method::POST,
        "/api/v1/rooms/test_room?password=test123",
        None,
    );

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
    let request1 = create_http_request(Method::POST, "/api/v1/rooms/duplicate_test", None);

    let response1 = app.clone().oneshot(request1).await?;
    assert_eq!(response1.status(), StatusCode::OK);

    // 第二次创建同名房间应该失败
    let request2 = create_http_request(Method::POST, "/api/v1/rooms/duplicate_test", None);

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
    let create_request = create_http_request(Method::POST, "/api/v1/rooms/delete_test", None);

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

    // 验证房间已被删除 - 根据业务逻辑，查找不存在的房间会自动创建
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
    let (app, _pool) = create_test_app().await?;

    let room_name = "workflow_test";

    // 1. 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=secret123", room_name),
        None,
    );

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

    // 5. 验证房间已删除 - 根据业务逻辑，查找不存在的房间会自动创建新房间
    let verify_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);

    let verify_response = app.clone().oneshot(verify_request).await?;
    assert_eq!(verify_response.status(), StatusCode::OK);

    let verify_body = axum::body::to_bytes(verify_response.into_body(), usize::MAX).await?;
    let verify_json: serde_json::Value = serde_json::from_slice(&verify_body)?;

    // 验证这是一个新创建的房间（ID 不同）
    assert_ne!(verify_json["id"].as_i64().unwrap(), room_id);
    assert_eq!(verify_json["name"].as_str().unwrap(), room_name);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_room_token_and_content_flow() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "content_test_room";

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=secret", room_name),
        None,
    );
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 签发 token
    let issue_payload = json!({ "password": "secret" });
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(issue_payload.to_string())),
    );
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let token_json: serde_json::Value = serde_json::from_slice(&body)?;
    let token = token_json["token"]
        .as_str()
        .expect("token string")
        .to_string();

    // 校验 token
    let validate_payload = json!({ "token": token });
    let validate_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens/validate", room_name),
        Some(Body::from(validate_payload.to_string())),
    );
    let validate_response = app.clone().oneshot(validate_request).await?;
    assert_eq!(validate_response.status(), StatusCode::OK);

    // 上传前预检
    let prepare_payload = json!({
        "files": [{
            "name": "hello.txt",
            "size": 11,
            "mime": "text/plain"
        }]
    });
    let prepare_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/contents/prepare?token={}",
            room_name, token
        ),
        Some(Body::from(prepare_payload.to_string())),
    );
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
            "/api/v1/rooms/{}/contents?token={}&reservation_id={}",
            room_name, token, reservation_id
        ))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(file_body))?;
    let upload_response = app.clone().oneshot(upload_request).await?;
    assert_eq!(upload_response.status(), StatusCode::OK);

    // 列出文件
    let list_request = create_http_request(
        Method::GET,
        &format!("/api/v1/rooms/{}/contents?token={}", room_name, token),
        None,
    );
    let list_response = app.clone().oneshot(list_request).await?;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = axum::body::to_bytes(list_response.into_body(), usize::MAX).await?;
    let list_json: serde_json::Value = serde_json::from_slice(&list_body)?;
    let items = list_json.as_array().expect("content list");
    assert!(!items.is_empty());
    let content_id = items[0]["id"].as_i64().expect("content id");

    // 下载文件
    let download_request = create_http_request(
        Method::GET,
        &format!(
            "/api/v1/rooms/{}/contents/{}?token={}",
            room_name, content_id, token
        ),
        None,
    );
    let download_response = app.clone().oneshot(download_request).await?;
    assert_eq!(download_response.status(), StatusCode::OK);

    // 删除文件
    let delete_payload = json!({ "ids": [content_id] });
    let delete_request = create_http_request(
        Method::DELETE,
        &format!("/api/v1/rooms/{}/contents?token={}", room_name, token),
        Some(Body::from(delete_payload.to_string())),
    );
    let delete_response = app.clone().oneshot(delete_request).await?;
    assert_eq!(delete_response.status(), StatusCode::OK);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_update_room_permissions_share_toggle() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "permission_test_room";

    // 创建房间（自动创建）
    let create_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 获取具有删除权限的 token（默认房间有所有权限）
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from("{}")),
    );
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token = issue_json["token"].as_str().unwrap().to_string();

    // 更新权限：只允许编辑，不允许分享和删除
    let permission_payload = json!({
        "edit": true,
        "share": false,
        "delete": false
    });
    let permission_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/permissions?token={}", room_name, token),
        Some(Body::from(permission_payload.to_string())),
    );
    let permission_response = app.clone().oneshot(permission_request).await?;

    // 先检查状态
    let status = permission_response.status();

    // 只有在状态不正确时才获取响应体用于调试
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(permission_response.into_body(), usize::MAX).await?;
        let response_text = String::from_utf8_lossy(&body);
        eprintln!("Permission update failed: {}", response_text);
        return Err(anyhow::anyhow!(
            "Permission update failed with status: {}",
            status
        ));
    }

    // 状态正确，获取响应体
    let permission_body = axum::body::to_bytes(permission_response.into_body(), usize::MAX).await?;
    let permission_json: serde_json::Value = serde_json::from_slice(&permission_body)?;

    // 当 share=false 时，slug 应该变成私有（包含 UUID）
    let new_slug = permission_json["slug"].as_str().expect("slug present");
    assert_ne!(new_slug, room_name);

    // 测试通过原始名称访问应该被禁止
    let original_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);
    let original_response = app.clone().oneshot(original_request).await?;
    assert_eq!(original_response.status(), StatusCode::UNAUTHORIZED); // 房间无法访问

    // 测试通过新的私有 slug 可以访问
    let private_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", new_slug), None);
    let private_response = app.clone().oneshot(private_request).await?;
    assert_eq!(private_response.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_token_refresh_revokes_old_token() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "refresh_test_room";

    // 创建房间（自动创建）
    let create_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 获取第一个 token
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from("{}")),
    );
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token1 = issue_json["token"].as_str().unwrap().to_string();

    // 使用第一个 token 获取新的 token（令牌续签）
    let refresh_payload = json!({ "token": token1 });
    let refresh_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(refresh_payload.to_string())),
    );
    let refresh_response = app.clone().oneshot(refresh_request).await?;
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body = axum::body::to_bytes(refresh_response.into_body(), usize::MAX).await?;
    let refresh_json: serde_json::Value = serde_json::from_slice(&refresh_body)?;
    let token2 = refresh_json["token"].as_str().unwrap().to_string();
    assert_ne!(token1, token2);

    // 验证新 token 有效
    let validate_new_payload = json!({ "token": token2 });
    let validate_new_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens/validate", room_name),
        Some(Body::from(validate_new_payload.to_string())),
    );
    let validate_new_response = app.clone().oneshot(validate_new_request).await?;
    assert_eq!(validate_new_response.status(), StatusCode::OK);

    // 验证旧 token 已失效
    let validate_old_payload = json!({ "token": token1 });
    let validate_old_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens/validate", room_name),
        Some(Body::from(validate_old_payload.to_string())),
    );
    let validate_old_response = app.clone().oneshot(validate_old_request).await?;
    assert_eq!(validate_old_response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}
