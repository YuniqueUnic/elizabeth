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

async fn issue_room_token(app: &axum::Router, room_identifier: &str) -> Result<String> {
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_identifier),
        Some(Body::from(json!({}).to_string())),
    );
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);

    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    Ok(issue_json["token"].as_str().unwrap().to_string())
}

async fn update_room_permissions(
    app: &axum::Router,
    room_identifier: &str,
    token: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value> {
    let permission_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/permissions?token={}",
            room_identifier, token
        ),
        Some(Body::from(payload.to_string())),
    );
    let permission_response = app.clone().oneshot(permission_request).await?;
    let status = permission_response.status();
    let permission_body = axum::body::to_bytes(permission_response.into_body(), usize::MAX).await?;

    if status != StatusCode::OK {
        let response_text = String::from_utf8_lossy(&permission_body);
        return Err(anyhow::anyhow!(
            "Permission update failed with status {}: {}",
            status,
            response_text
        ));
    }

    Ok(serde_json::from_slice(&permission_body)?)
}

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
async fn test_private_slug_stays_stable_when_delete_permission_changes() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "permission_private_delete_room";

    let create_request =
        create_http_request(Method::POST, &format!("/api/v1/rooms/{}", room_name), None);
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    let public_token = issue_room_token(&app, room_name).await?;
    let private_room = update_room_permissions(
        &app,
        room_name,
        &public_token,
        json!({
            "edit": true,
            "share": false,
            "delete": true
        }),
    )
    .await?;
    let private_slug = private_room["slug"].as_str().expect("private slug");
    assert_ne!(private_slug, room_name);

    let private_token = issue_room_token(&app, private_slug).await?;
    let updated_room = update_room_permissions(
        &app,
        private_slug,
        &private_token,
        json!({
            "edit": true,
            "share": true,
            "delete": false
        }),
    )
    .await?;

    assert_eq!(updated_room["slug"].as_str(), Some(private_slug));
    let permission = updated_room["permission"]
        .as_str()
        .expect("permission string");
    assert!(
        !permission.contains("DELETE"),
        "delete permission must be removed"
    );

    let public_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);
    let public_response = app.clone().oneshot(public_request).await?;
    assert_eq!(public_response.status(), StatusCode::UNAUTHORIZED);

    let private_request = create_http_request(
        Method::GET,
        &format!("/api/v1/rooms/{}", private_slug),
        None,
    );
    let private_response = app.clone().oneshot(private_request).await?;
    assert_eq!(private_response.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_private_slug_stays_stable_when_edit_permission_changes() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "permission_private_edit_room";

    let create_request =
        create_http_request(Method::POST, &format!("/api/v1/rooms/{}", room_name), None);
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    let public_token = issue_room_token(&app, room_name).await?;
    let private_room = update_room_permissions(
        &app,
        room_name,
        &public_token,
        json!({
            "edit": true,
            "share": false,
            "delete": true
        }),
    )
    .await?;
    let private_slug = private_room["slug"].as_str().expect("private slug");
    assert_ne!(private_slug, room_name);

    let private_token = issue_room_token(&app, private_slug).await?;
    let updated_room = update_room_permissions(
        &app,
        private_slug,
        &private_token,
        json!({
            "edit": false,
            "share": true,
            "delete": false
        }),
    )
    .await?;

    assert_eq!(updated_room["slug"].as_str(), Some(private_slug));
    let permission = updated_room["permission"]
        .as_str()
        .expect("permission string");
    assert!(
        !permission.contains("EDITABLE"),
        "edit permission must be removed"
    );
    assert!(permission.contains("SHARE"), "share permission must remain");

    let public_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);
    let public_response = app.clone().oneshot(public_request).await?;
    assert_eq!(public_response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}
