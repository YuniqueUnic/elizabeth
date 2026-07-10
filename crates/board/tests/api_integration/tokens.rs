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

#[tokio::test]
async fn test_token_refresh_revokes_old_token() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "refresh_test_room";

    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}", room_name),
        Some(Body::from("{}")),
    );
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 获取访问令牌和刷新令牌
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(
            json!({ "with_refresh_token": true }).to_string(),
        )),
    );
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token1 = issue_json["token"].as_str().unwrap().to_string();
    let refresh_token = issue_json["refresh_token"].as_str().unwrap().to_string();

    // 使用刷新令牌获取新的访问令牌和刷新令牌
    let refresh_payload = json!({ "refresh_token": refresh_token });
    let refresh_request = create_http_request(
        Method::POST,
        "/api/v1/auth/refresh",
        Some(Body::from(refresh_payload.to_string())),
    );
    let refresh_response = app.clone().oneshot(refresh_request).await?;
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body = axum::body::to_bytes(refresh_response.into_body(), usize::MAX).await?;
    let refresh_json: serde_json::Value = serde_json::from_slice(&refresh_body)?;
    let token2 = refresh_json["access_token"].as_str().unwrap().to_string();
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
