//! 认证功能测试
//!
//! 测试用户认证、登出和令牌管理功能

mod common;

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, StatusCode},
    response::IntoResponse,
};
use serde_json::json;
use sqlx::SqlitePool;
use tower::ServiceExt;

use common::{
    create_test_app,
    fixtures::{passwords, room_names},
    http::{assert_json, assert_status, create_request as create_http_request, send_request},
};

use board::route::auth::auth_router;

/// 测试用户登出功能
#[tokio::test]
async fn test_user_logout() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试使用认证头的登出
    let logout_request = create_http_request(
        Method::POST,
        "/api/v1/auth/logout",
        Some(Body::from(
            json!({ "user_id": "test_user_123" }).to_string(),
        )),
    );

    let logout_response = app.clone().oneshot(logout_request).await?;
    assert_status(&logout_response, StatusCode::OK);

    let logout_body = axum::body::to_bytes(logout_response.into_body(), usize::MAX).await?;
    let logout_json: serde_json::Value = serde_json::from_slice(&logout_body)?;
    assert!(logout_json["success"].is_boolean());
    assert_eq!(logout_json["user_id"].as_str().unwrap(), "test_user_123");

    Ok(())
}

/// 测试不使用认证头的登出功能
#[tokio::test]
async fn test_logout_without_auth_header() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试不使用认证头的登出
    let logout_request = create_http_request(
        Method::POST,
        "/api/v1/auth/logout",
        Some(Body::from(
            json!({ "user_id": "test_user_456" }).to_string(),
        )),
    );

    let logout_response = app.clone().oneshot(logout_request).await?;
    assert_status(&logout_response, StatusCode::UNAUTHORIZED);

    Ok(())
}

/// 测试通过访问令牌获取房间
#[tokio::test]
async fn test_room_access_token() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        "/api/v1/rooms/token_access_test?password=token123",
        None,
    );

    let create_response = app.clone().oneshot(create_request).await?;
    assert_status(&create_response, StatusCode::OK);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let create_json: serde_json::Value = serde_json::from_slice(&create_body)?;
    let access_token = create_json["access_token"]
        .as_str()
        .expect("access token")
        .to_string();

    // 使用访问令牌获取房间
    let access_request = create_http_request(
        Method::GET,
        &format!(
            "/api/v1/rooms/token_access_test?access_token={}",
            access_token
        ),
        None,
    );

    let access_response = app.clone().oneshot(access_request).await?;
    assert_status(&access_response, StatusCode::OK);

    let access_body = axum::body::to_bytes(access_response.into_body(), usize::MAX).await?;
    let access_json: serde_json::Value = serde_json::from_slice(&access_body)?;
    assert_eq!(access_json["name"].as_str().unwrap(), "token_access_test");
    assert!(access_json["id"].is_number());

    Ok(())
}

/// 测试无效访问令牌
#[tokio::test]
async fn test_invalid_room_access_token() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 使用无效令牌尝试访问房间
    let access_request = create_http_request(
        Method::GET,
        "/api/v1/rooms/protected_room?access_token=invalid_token",
        None,
    );

    let access_response = app.clone().oneshot(access_request).await?;
    assert_status(&access_response, StatusCode::UNAUTHORIZED);

    Ok(())
}

/// 测试令牌刷新功能
#[tokio::test]
async fn test_token_refresh() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        "/api/v1/rooms/token_refresh_test?password=refresh123",
        None,
    );

    let create_response = app.clone().oneshot(create_request).await?;
    assert_status(&create_response, StatusCode::OK);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let create_json: serde_json::Value = serde_json::from_slice(&create_body)?;
    let access_token = create_json["access_token"]
        .as_str()
        .expect("access token")
        .to_string();

    // 测试令牌刷新
    let refresh_payload = json!({ "refresh_token": "valid_refresh_token" });
    let refresh_request = create_http_request(
        Method::POST,
        "/api/v1/rooms/token_refresh_test/tokens/refresh",
        Some(Body::from(refresh_payload.to_string())),
    );

    let refresh_response = app.clone().oneshot(refresh_request).await?;
    assert_status(&refresh_response, StatusCode::OK);

    let refresh_body = axum::body::to_bytes(refresh_response.into_body(), usize::MAX).await?;
    let refresh_json: serde_json::Value = serde_json::from_slice(&refresh_body)?;
    let new_access_token = refresh_json["access_token"]
        .as_str()
        .expect("new access token")
        .to_string();

    // 验证旧令牌失效
    let old_access_request = create_http_request(
        Method::POST,
        "/api/v1/rooms/token_refresh_test/tokens/validate",
        Some(Body::from(
            json!({ "access_token": access_token }).to_string(),
        )),
    );

    let old_access_response = app.clone().oneshot(old_access_request).await?;
    assert_status(&old_access_response, StatusCode::UNAUTHORIZED);

    // 验证新令牌有效
    let new_access_request = create_http_request(
        Method::POST,
        "/api/v1/rooms/token_refresh_test/tokens/validate",
        Some(Body::from(
            json!({ "access_token": new_access_token }).to_string(),
        )),
    );

    let new_access_response = app.clone().oneshot(new_access_request).await?;
    assert_status(&new_access_response, StatusCode::OK);

    Ok(())
}

/// 测试认证中间件
#[tokio::test]
async fn test_auth_middleware() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试受保护的路由
    let protected_request = create_http_request(
        Method::GET,
        "/api/v1/rooms/middleware_test",
        Some(Body::from(
            json!({ "user_id": "test_user_789" }).to_string(),
        )),
    );

    let protected_response = app.clone().oneshot(protected_request).await?;
    assert_status(&protected_response, StatusCode::OK);

    let protected_body = axum::body::to_bytes(protected_response.into_body(), usize::MAX).await?;
    let protected_json: serde_json::Value = serde_json::from_slice(&protected_body)?;
    assert_eq!(
        protected_json["message"].as_str().unwrap(),
        "Authenticated access granted"
    );

    // 测试未认证的访问
    let unauthorized_request =
        create_http_request(Method::GET, "/api/v1/rooms/middleware_test", None);

    let unauthorized_response = app.clone().oneshot(unauthorized_request).await?;
    assert_status(&unauthorized_response, StatusCode::UNAUTHORIZED);

    Ok(())
}

/// 测试并发认证请求
#[tokio::test]
async fn test_concurrent_auth_requests() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 发送多个并发请求
    let responses = futures::future::join_all((0..5).map(|i| {
        let app_clone = app.clone();
        async move {
            let request = create_http_request(
                Method::GET,
                &format!("/api/v1/rooms/concurrent_test_{}", i),
                Some(Body::from(
                    json!({ "user_id": format!("user_{}", i) }).to_string(),
                )),
            );
            app_clone.oneshot(request).await
        }
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;

    // 验证所有响应
    for (i, response) in responses.into_iter().enumerate() {
        assert_status(&response, StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let response_json: serde_json::Value = serde_json::from_slice(&body)?;

        assert_eq!(
            response_json["user_id"].as_str().unwrap(),
            format!("user_{}", i)
        );
    }

    Ok(())
}

/// 测试认证错误处理
#[tokio::test]
async fn test_auth_error_handling() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试无效的 JSON
    let invalid_json_request = create_http_request(
        Method::POST,
        "/api/v1/auth/logout",
        Some(Body::from("{ invalid json }".to_string())),
    );

    let invalid_json_response = app.clone().oneshot(invalid_json_request).await?;
    assert_status(&invalid_json_response, StatusCode::BAD_REQUEST);

    // 测试空的用户 ID
    let empty_user_request = create_http_request(
        Method::POST,
        "/api/v1/auth/logout",
        Some(Body::from(json!({ "user_id": "" }).to_string())),
    );

    let empty_user_response = app.clone().oneshot(empty_user_request).await?;
    assert_status(&empty_user_response, StatusCode::BAD_REQUEST);

    Ok(())
}
