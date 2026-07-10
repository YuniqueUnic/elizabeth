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
    fixtures::{passwords, room_names},
    http::{assert_json, create_request as create_http_request},
};

fn create_room_request(room_name: &str, password: Option<&str>) -> axum::http::Request<Body> {
    let payload = match password {
        Some(password) => json!({ "password": password }),
        None => json!({}),
    };
    create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{room_name}"),
        Some(Body::from(payload.to_string())),
    )
}

#[tokio::test]
async fn test_room_token_issue_no_password() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "no_password_room";

    let create_request = create_room_request(room_name, None);
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 签发令牌（无需密码）
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

    assert!(token_json["token"].is_string());
    assert!(token_json["claims"].is_object());
    assert!(token_json["expires_at"].is_string());
    assert!(token_json["refresh_token"].is_null()); // 默认不请求刷新令牌

    Ok(())
}

/// 测试房间令牌签发 - 有密码房间
#[tokio::test]
async fn test_room_token_issue_with_password() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "password_room";
    let password = "secret123";

    // 创建带密码的房间
    let create_request = create_room_request(room_name, Some(password));
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 使用正确密码签发令牌
    let token_payload = json!({ "password": password });
    let token_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(token_payload.to_string())),
    );

    let token_response = app.clone().oneshot(token_request).await?;
    assert_eq!(token_response.status(), StatusCode::OK);

    let token_body = axum::body::to_bytes(token_response.into_body(), usize::MAX).await?;
    let token_json: serde_json::Value = serde_json::from_slice(&token_body)?;

    assert!(token_json["token"].is_string());
    assert!(token_json["claims"].is_object());

    Ok(())
}

/// 测试房间令牌签发 - 错误密码
#[tokio::test]
async fn test_room_token_issue_wrong_password() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "wrong_password_room";
    let password = "correct123";

    // 创建带密码的房间
    let create_request = create_room_request(room_name, Some(password));
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 使用错误密码尝试签发令牌
    let token_payload = json!({ "password": "wrong_password" });
    let token_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(token_payload.to_string())),
    );

    let token_response = app.clone().oneshot(token_request).await?;
    assert_eq!(token_response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

/// 测试房间令牌验证
#[tokio::test]
async fn test_room_token_validation() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "validation_room";

    let create_request = create_room_request(room_name, None);
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

    // 验证令牌
    let validate_payload = json!({ "token": token });
    let validate_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens/validate", room_name),
        Some(Body::from(validate_payload.to_string())),
    );

    let validate_response = app.clone().oneshot(validate_request).await?;
    assert_eq!(validate_response.status(), StatusCode::OK);

    let validate_body = axum::body::to_bytes(validate_response.into_body(), usize::MAX).await?;
    let validate_json: serde_json::Value = serde_json::from_slice(&validate_body)?;

    assert!(validate_json["claims"].is_object());
    assert_eq!(validate_json["claims"]["room_name"], room_name);

    Ok(())
}

/// 测试无效令牌验证
#[tokio::test]
async fn test_invalid_room_token_validation() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "invalid_token_room";

    let create_request = create_room_request(room_name, None);
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 验证无效令牌
    let validate_payload = json!({ "token": "invalid_token_string" });
    let validate_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens/validate", room_name),
        Some(Body::from(validate_payload.to_string())),
    );

    let validate_response = app.clone().oneshot(validate_request).await?;
    assert_eq!(validate_response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}
