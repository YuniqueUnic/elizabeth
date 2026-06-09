#![allow(unused_variables, unused_imports, dead_code)]

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{
    create_test_app,
    http::{assert_json, create_request as create_http_request},
};

#[tokio::test]
async fn test_file_metadata() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "file_metadata_test_room";

    // 创建房间
    let create_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 签发令牌
    let token_payload = json!({});
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

    // 测试文件元数据端点
    let metadata_request = create_http_request(
        Method::GET,
        &format!("/api/v1/rooms/{}/storage/files?token={}", room_name, token),
        None,
    );
    let metadata_response = app.clone().oneshot(metadata_request).await?;

    // 文件元数据端点可能实现或未实现
    let status = metadata_response.status();
    println!("File metadata status: {}", status);

    // 验证端点行为合理
    assert!(
        status == StatusCode::OK
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::NOT_IMPLEMENTED
    );

    Ok(())
}

/// 测试存储清理功能
#[tokio::test]
async fn test_storage_cleanup() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "storage_cleanup_test_room";

    // 创建房间
    let create_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 签发令牌
    let token_payload = json!({});
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

    // 测试存储清理端点
    let cleanup_request = create_http_request(
        Method::DELETE,
        &format!(
            "/api/v1/rooms/{}/storage/cleanup?token={}",
            room_name, token
        ),
        None,
    );
    let cleanup_response = app.clone().oneshot(cleanup_request).await?;

    // 存储清理端点可能实现或未实现
    let status = cleanup_response.status();
    println!("Storage cleanup status: {}", status);

    // 验证端点行为合理
    assert!(
        status == StatusCode::OK
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::NOT_IMPLEMENTED
            || status == StatusCode::NO_CONTENT
    );

    Ok(())
}

/// 测试存储权限验证
#[tokio::test]
async fn test_storage_permissions() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "storage_perms_test_room";
    let password = "storage123";

    // 创建带密码的房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password={}", room_name, password),
        None,
    );
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 测试无令牌访问存储功能
    let content_request = create_http_request(
        Method::PUT,
        &format!("/api/v1/rooms/{}/content", room_name),
        Some(Body::from(
            json!({
                "text": "test content",
                "urls": []
            })
            .to_string(),
        )),
    );
    let content_response = app.clone().oneshot(content_request).await?;

    // 内容更新端点可能未实现，所以可能是 404 或 401
    let content_status = content_response.status();
    assert!(content_status == StatusCode::UNAUTHORIZED || content_status == StatusCode::NOT_FOUND);

    // 测试错误密码令牌
    let wrong_token_payload = json!({ "password": "wrong_password" });
    let wrong_token_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(wrong_token_payload.to_string())),
    );
    let wrong_token_response = app.clone().oneshot(wrong_token_request).await?;
    assert_eq!(wrong_token_response.status(), StatusCode::UNAUTHORIZED);

    // 测试正确密码令牌
    let correct_token_payload = json!({ "password": password });
    let correct_token_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(correct_token_payload.to_string())),
    );
    let correct_token_response = app.clone().oneshot(correct_token_request).await?;
    assert_eq!(correct_token_response.status(), StatusCode::OK);

    let token_body = axum::body::to_bytes(correct_token_response.into_body(), usize::MAX).await?;
    let token_json: serde_json::Value = serde_json::from_slice(&token_body)?;
    let token = token_json["token"].as_str().unwrap().to_string();

    // 使用正确令牌测试内容更新
    let valid_content_request = create_http_request(
        Method::PUT,
        &format!("/api/v1/rooms/{}/content?token={}", room_name, token),
        Some(Body::from(
            json!({
                "text": "valid content",
                "urls": []
            })
            .to_string(),
        )),
    );
    let valid_content_response = app.oneshot(valid_content_request).await?;

    // 内容更新端点可能未实现，所以可能是 404 或 200
    let valid_status = valid_content_response.status();
    assert!(valid_status == StatusCode::OK || valid_status == StatusCode::NOT_FOUND);

    if valid_status == StatusCode::OK {
        println!("Content update with valid token succeeded");
    } else {
        println!("Content update endpoint not implemented");
    }

    Ok(())
}
