//! 分块上传功能测试
//!
//! 测试文件分块上传的完整工作流程

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
    fixtures::{file_sizes, filenames, passwords, room_names},
    http::{assert_json, assert_status, create_request as create_http_request, send_request},
};

use board::route::room::api_router;

/// 测试基本的分块上传流程
#[tokio::test]
async fn test_chunked_upload_complete_workflow() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 1. 创建房间
    let room_name = "chunked_upload_test_room";
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=chunked123", room_name),
        None,
    );

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 2. 发放访问令牌
    let issue_payload = json!({ "password": "chunked123" });
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(issue_payload.to_string())),
    );

    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);

    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token = issue_json["token"]
        .as_str()
        .expect("token string")
        .to_string();

    // 3. 预检上传
    let prepare_payload = json!({
        "files": [{
            "name": filenames::LARGE_FILE,
            "size": file_sizes::LARGE,
            "mime": "application/zip"
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

    // 4. 分块上传
    let chunk1 = "chunk1_data";
    let chunk2 = "chunk2_data";
    let boundary = "----chunked-upload-boundary";

    // 第一块
    let chunk1_body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk\"; filename=\"chunk1\"\r\n\
         Content-Type: application/octet-stream\r\n\
         \r\n\
         {}\r\n\
         --{boundary}--\r\n",
        chunk1
    );

    let upload1_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/contents/chunk?token={}&reservation_id={}",
            room_name, token, reservation_id
        ),
        Some(Body::from(chunk1_body)),
    );

    let upload1_response = app.clone().oneshot(upload1_request).await?;
    assert_eq!(upload1_response.status(), StatusCode::OK);

    // 第二块
    let chunk2_body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk\"; filename=\"chunk2\"\r\n\
         Content-Type: application/octet-stream\r\n\
         \r\n\
         {}\r\n\
         --{boundary}--\r\n",
        chunk2
    );

    let upload2_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/contents/chunk?token={}&reservation_id={}",
            room_name, token, reservation_id
        ),
        Some(Body::from(chunk2_body)),
    );

    let upload2_response = app.clone().oneshot(upload2_request).await?;
    assert_eq!(upload2_response.status(), StatusCode::OK);

    // 5. 完成上传
    let complete_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/contents/complete?token={}&reservation_id={}",
            room_name, token, reservation_id
        ),
        Some(Body::empty()),
    );

    let complete_response = app.clone().oneshot(complete_request).await?;
    assert_eq!(complete_response.status(), StatusCode::OK);

    // 6. 验证文件列表包含上传的文件
    let list_request = create_http_request(
        Method::GET,
        &format!("/api/v1/rooms/{}/contents?token={}", room_name, token),
        None,
    );

    let list_response = app.clone().oneshot(list_request).await?;
    assert_eq!(list_response.status(), StatusCode::OK);

    let list_body = axum::body::to_bytes(list_response.into_body(), usize::MAX).await?;
    let list_json: serde_json::Value = serde_json::from_slice(&list_body)?;
    let contents = list_json.as_array().expect("contents array");

    assert_eq!(contents.len(), 1);
    let uploaded_file = &contents[0];
    assert_eq!(
        uploaded_file["name"].as_str().unwrap(),
        filenames::LARGE_FILE
    );
    assert_eq!(
        uploaded_file["size"].as_i64().unwrap(),
        file_sizes::LARGE as i64
    );

    Ok(())
}

/// 测试并发分块上传
#[tokio::test]
async fn test_concurrent_chunked_upload() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 创建房间和令牌（简化流程）
    let room_name = "concurrent_upload_test";

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=concurrent123", room_name),
        None,
    );

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 获取房间详情（包含自动创建的访问令牌）
    let room_request =
        create_http_request(Method::GET, &format!("/api/v1/rooms/{}", room_name), None);

    let room_response = app.clone().oneshot(room_request).await?;
    assert_eq!(room_response.status(), StatusCode::OK);

    let room_body = axum::body::to_bytes(room_response.into_body(), usize::MAX).await?;
    let room_json: serde_json::Value = serde_json::from_slice(&room_body)?;
    let token = room_json["access_token"]
        .as_str()
        .expect("access token")
        .to_string();

    // 并发多个分块上传
    let boundary = "----concurrent-boundary";
    let chunk_data = "concurrent_test_data";

    // 并发上传请求
    let upload_responses = futures::future::join_all((0..5).map(|i| {
        let app_clone = app.clone();
        let boundary_clone = boundary.clone();
        let chunk_data_clone = chunk_data.to_string();
        let room_name_clone = room_name.clone();
        let token_clone = token.clone();

        async move {
            let chunk_body = format!(
                "--{boundary}\r\n\
                     Content-Disposition: form-data; name=\"chunk\"; filename=\"chunk{}\"\r\n\
                     Content-Type: application/octet-stream\r\n\
                     \r\n\
                     {}\r\n\
                     --{boundary}--\r\n",
                i, chunk_data_clone
            );

            let request = create_http_request(
                Method::POST,
                &format!(
                    "/api/v1/rooms/{}/contents/chunk?token={}",
                    room_name_clone, token_clone
                ),
                Some(Body::from(chunk_body)),
            );
            app_clone.oneshot(request).await
        }
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;

    // 验证所有上传都成功
    for (i, response) in upload_responses.into_iter().enumerate() {
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Chunk {} upload failed",
            i
        );
    }

    Ok(())
}

/// 测试分块上传错误处理
#[tokio::test]
async fn test_chunked_upload_error_handling() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    // 测试无效令牌
    let request = create_http_request(
        Method::POST,
        "/api/v1/rooms/test_room/contents/chunk?token=invalid_token",
        Some(Body::from("test data")),
    );

    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 测试无效预约 ID
    let request2 = create_http_request(
        Method::POST,
        "/api/v1/rooms/test_room/contents/chunk?token=valid_token&reservation_id=invalid",
        Some(Body::from("test data")),
    );

    let response2 = app.oneshot(request2).await?;
    assert_eq!(response2.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

/// 测试分块上传大小限制
#[tokio::test]
async fn test_chunked_upload_size_limits() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "size_limit_test";

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=limit123", room_name),
        None,
    );

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 获取访问令牌
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(r#"{"password": "limit123"}"#)),
    );

    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);

    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token = issue_json["token"]
        .as_str()
        .expect("token string")
        .to_string();

    // 尝试上传超大文件
    let large_data = "x".repeat(100 * 1024 * 1024); // 100MB
    let prepare_payload = json!({
        "files": [{
            "name": "huge_file.dat",
            "size": large_data.len(),
            "mime": "application/octet-stream"
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
    assert_eq!(prepare_response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

/// 测试分块上传超时处理
#[tokio::test]
async fn test_chunked_upload_timeout() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "timeout_test";

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}?password=timeout123", room_name),
        None,
    );

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 获取访问令牌
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(r#"{"password": "timeout123"}"#)),
    );

    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);

    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token = issue_json["token"]
        .as_str()
        .expect("token string")
        .to_string();

    // 测试预约过期后的上传
    let boundary = "----timeout-boundary";
    let chunk_data = "timeout_test_data";

    let chunk_body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk\"; filename=\"timeout_chunk\"\r\n\
         Content-Type: application/octet-stream\r\n\
         \r\n\
         {}\r\n\
         --{boundary}--\r\n",
        chunk_data
    );

    let upload_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/contents/chunk?token={}", room_name, token),
        Some(Body::from(chunk_body)),
    );

    // 模拟等待时间超过预约 TTL
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let upload_response = app.oneshot(upload_request).await?;
    assert_eq!(upload_response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}
