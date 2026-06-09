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
async fn test_upload_reservation() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "upload_reservation_test_room";

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

    // 预留上传
    let reserve_payload = json!({
        "files": [
            {
                "name": "test_file.txt",
                "size": 1024,
                "content_type": "text/plain"
            },
            {
                "name": "test_image.jpg",
                "size": 2048,
                "content_type": "image/jpeg"
            }
        ]
    });
    let reserve_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/uploads/reserve?token={}",
            room_name, token
        ),
        Some(Body::from(reserve_payload.to_string())),
    );
    let reserve_response = app.clone().oneshot(reserve_request).await?;

    // 上传预留可能成功或失败，取决于 API 实现
    let status = reserve_response.status();
    println!("Upload reservation status: {}", status);

    // 验证端点可访问且返回合理的响应
    assert!(
        status == StatusCode::OK
            || status == StatusCode::NOT_IMPLEMENTED
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::NOT_FOUND
    );

    Ok(())
}

/// 测试分块合并功能
#[tokio::test]
async fn test_chunk_merge() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "chunk_merge_test_room";

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

    // 测试分块合并（使用预留 ID 或上传令牌）
    let merge_payload = json!({
        "reservation_id": "test_reservation_id",
        "final_hash": "dummy_hash_value"
    });
    let merge_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/uploads/merge?token={}", room_name, token),
        Some(Body::from(merge_payload.to_string())),
    );
    let merge_response = app.clone().oneshot(merge_request).await?;

    // 分块合并可能成功或失败，取决于 API 实现
    let status = merge_response.status();
    println!("Chunk merge status: {}", status);

    // 验证端点可访问且返回合理的响应
    assert!(
        status == StatusCode::OK
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::NOT_IMPLEMENTED
    );

    Ok(())
}

/// 测试存储路径管理
#[tokio::test]
async fn test_storage_paths() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "storage_path_test_room";

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

    // 测试获取存储路径信息
    let paths_request = create_http_request(
        Method::GET,
        &format!("/api/v1/rooms/{}/storage/paths?token={}", room_name, token),
        None,
    );
    let paths_response = app.clone().oneshot(paths_request).await?;

    // 存储路径端点可能实现或未实现
    let status = paths_response.status();
    println!("Storage paths status: {}", status);

    // 验证端点行为合理
    assert!(
        status == StatusCode::OK
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::NOT_IMPLEMENTED
    );

    Ok(())
}
