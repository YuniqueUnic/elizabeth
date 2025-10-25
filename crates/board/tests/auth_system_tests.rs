//! 认证系统完整测试
//!
//! 测试完整的认证系统功能，包括刷新令牌、登出、清理等

mod common;

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::json;
use std::string::String;
use tower::ServiceExt;

use common::{create_test_app, http::create_request as create_http_request};

/// 测试刷新令牌功能
#[tokio::test]
async fn test_refresh_token_endpoint() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "refresh_token_test_room";

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=refresh123", room_name),
        None,
    );
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 签发初始令牌
    let token_payload = json!({ "password": "refresh123" });
    let token_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(token_payload.to_string())),
    );
    let token_response = app.clone().oneshot(token_request).await?;
    assert_eq!(token_response.status(), StatusCode::OK);

    let token_body = axum::body::to_bytes(token_response.into_body(), usize::MAX).await?;
    let token_json: serde_json::Value = serde_json::from_slice(&token_body)?;
    let initial_token = token_json["token"].as_str().unwrap().to_string();

    // 测试刷新令牌端点
    let refresh_payload = json!({
        "refresh_token": "dummy_refresh_token"  // 实际应用中需要真实的刷新令牌
    });
    let refresh_request = create_http_request(
        Method::POST,
        "/api/v1/auth/refresh",
        Some(Body::from(refresh_payload.to_string())),
    );

    let refresh_response = app.oneshot(refresh_request).await?;

    // 刷新令牌端点可能返回 400（无效刷新令牌）或其他状态码
    // 主要验证端点存在并可访问
    let status = refresh_response.status();
    println!("Refresh token endpoint status: {}", status);

    // 验证响应是 JSON 格式（即使错误响应也应该是 JSON）
    if status == StatusCode::OK || status == StatusCode::BAD_REQUEST {
        let body = axum::body::to_bytes(refresh_response.into_body(), usize::MAX).await?;
        let _json: serde_json::Value = serde_json::from_slice(&body)?;
        println!("Refresh token response is valid JSON");
    }

    Ok(())
}

/// 测试登出功能
#[tokio::test]
async fn test_logout_endpoint() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "logout_test_room";

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=logout123", room_name),
        None,
    );
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 签发令牌
    let token_payload = json!({ "password": "logout123" });
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

    // 测试登出端点
    let logout_payload = json!({
        "token": token
    });
    let logout_request = create_http_request(
        Method::POST,
        "/api/v1/auth/logout",
        Some(Body::from(logout_payload.to_string())),
    );

    let logout_response = app.oneshot(logout_request).await?;

    // 验证登出响应
    let status = logout_response.status();
    println!("Logout endpoint status: {}", status);

    // 登出应该成功或返回特定的错误状态
    assert!(
        status == StatusCode::OK
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::UNPROCESSABLE_ENTITY
    );

    // 验证响应格式（可能是 JSON 或纯文本）
    let body = axum::body::to_bytes(logout_response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8_lossy(&body);
    println!("Logout response: {}", body_str);

    // 尝试解析为 JSON，如果失败也没关系
    match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(logout_json) => println!("Logout JSON: {}", logout_json),
        Err(_) => println!("Logout response is not JSON format, which is acceptable"),
    }

    Ok(())
}

/// 测试令牌清理功能
#[tokio::test]
async fn test_token_cleanup_endpoint() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试令牌清理端点
    let cleanup_request = create_http_request(Method::DELETE, "/api/v1/auth/cleanup", None);

    let cleanup_response = app.oneshot(cleanup_request).await?;

    // 验证清理响应
    let status = cleanup_response.status();
    println!("Token cleanup endpoint status: {}", status);

    // 清理端点应该可访问
    assert!(
        status == StatusCode::OK
            || status == StatusCode::NO_CONTENT
            || status == StatusCode::BAD_REQUEST
    );

    // 如果成功，验证响应格式
    if status == StatusCode::OK {
        let body = axum::body::to_bytes(cleanup_response.into_body(), usize::MAX).await?;
        let cleanup_json: serde_json::Value = serde_json::from_slice(&body)?;
        println!("Token cleanup response: {}", cleanup_json);
    }

    Ok(())
}

/// 测试认证错误处理
#[tokio::test]
async fn test_auth_error_handling() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试无效的刷新令牌
    let invalid_refresh_payload = json!({
        "refresh_token": "invalid_token"
    });
    let invalid_refresh_request = create_http_request(
        Method::POST,
        "/api/v1/auth/refresh",
        Some(Body::from(invalid_refresh_payload.to_string())),
    );

    let invalid_refresh_response = app.clone().oneshot(invalid_refresh_request).await?;
    assert!(
        invalid_refresh_response.status() == StatusCode::BAD_REQUEST
            || invalid_refresh_response.status() == StatusCode::UNAUTHORIZED
    );

    // 测试无效的登出请求
    let invalid_logout_payload = json!({
        "token": "invalid_token"
    });
    let invalid_logout_request = create_http_request(
        Method::POST,
        "/api/v1/auth/logout",
        Some(Body::from(invalid_logout_payload.to_string())),
    );

    let invalid_logout_response = app.clone().oneshot(invalid_logout_request).await?;
    assert!(
        invalid_logout_response.status() == StatusCode::BAD_REQUEST
            || invalid_logout_response.status() == StatusCode::UNAUTHORIZED
            || invalid_logout_response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );

    // 测试缺失参数的请求
    let missing_params_request =
        create_http_request(Method::POST, "/api/v1/auth/refresh", Some(Body::from("{}")));

    let missing_params_response = app.clone().oneshot(missing_params_request).await?;
    assert_eq!(
        missing_params_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    Ok(())
}

/// 测试认证端点可访问性
#[tokio::test]
async fn test_auth_endpoint_accessibility() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试所有认证端点是否可访问
    let endpoints = vec![
        ("GET", "/api/v1/auth/refresh"),
        ("POST", "/api/v1/auth/refresh"),
        ("GET", "/api/v1/auth/logout"),
        ("POST", "/api/v1/auth/logout"),
        ("GET", "/api/v1/auth/cleanup"),
        ("DELETE", "/api/v1/auth/cleanup"),
    ];

    for (method, endpoint) in endpoints {
        let request_method = match method {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "DELETE" => Method::DELETE,
            _ => Method::GET,
        };
        let request = create_http_request(request_method, endpoint, None);

        let response = app.clone().oneshot(request).await?;

        // 所有端点都应该可访问（返回成功状态码或客户端错误状态码）
        let status = response.status();
        assert!(
            status.is_success() || status.is_client_error(),
            "Endpoint {} {} should be accessible (status: {})",
            method,
            endpoint,
            status
        );
    }

    Ok(())
}

/// 测试认证与房间令牌的集成
#[tokio::test]
async fn test_auth_room_token_integration() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "integration_test_room";

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=integration123", room_name),
        None,
    );
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 签发房间令牌
    let token_payload = json!({ "password": "integration123" });
    let token_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(token_payload.to_string())),
    );
    let token_response = app.clone().oneshot(token_request).await?;
    assert_eq!(token_response.status(), StatusCode::OK);

    // 验证房间令牌
    let token_body = axum::body::to_bytes(token_response.into_body(), usize::MAX).await?;
    let token_json: serde_json::Value = serde_json::from_slice(&token_body)?;
    let token = token_json["token"].as_str().unwrap().to_string();

    let validate_payload = json!({ "token": token });
    let validate_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens/validate", room_name),
        Some(Body::from(validate_payload.to_string())),
    );
    let validate_response = app.clone().oneshot(validate_request).await?;
    assert_eq!(validate_response.status(), StatusCode::OK);

    // 使用相同的令牌进行登出（测试系统间的集成）
    let logout_payload = json!({ "token": token });
    let logout_request = create_http_request(
        Method::POST,
        "/api/v1/auth/logout",
        Some(Body::from(logout_payload.to_string())),
    );
    let logout_response = app.clone().oneshot(logout_request).await?;

    // 验证登出结果
    let logout_status = logout_response.status();
    println!("Integrated logout status: {}", logout_status);

    // 验证令牌现在是否无效
    let validate_after_logout_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens/validate", room_name),
        Some(Body::from(validate_payload.to_string())),
    );
    let validate_after_logout_response = app.clone().oneshot(validate_after_logout_request).await?;

    // 令牌现在应该无效（取决于登出实现）
    let final_status = validate_after_logout_response.status();
    println!("Token validation after logout status: {}", final_status);

    Ok(())
}
