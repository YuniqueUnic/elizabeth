#![allow(unused_variables, unused_imports, dead_code)]
//! 分块上传功能测试
//!
//! 测试文件分块上传的完整工作流程

mod common;

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use common::{
    create_test_app,
    fixtures::{file_sizes, filenames, passwords, room_names},
    http::{assert_json, assert_status, create_request as create_http_request, send_request},
};

use board::route::room::api_router;

fn create_room_request(room_name: &str, password: Option<&str>) -> Request<Body> {
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

/// 测试基本的分块上传流程
#[tokio::test]
async fn test_chunked_upload_complete_workflow() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "chunked_upload_test_room";

    // 1. 创建房间
    let create_request = create_room_request(room_name, Some("chunked123"));

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

    // 3. 预检上传 - 使用正确的 API 端点
    let file_data = "Hello, World! This is test data for chunked upload.";
    let prepare_payload = json!({
        "files": [{
            "name": "test_file.txt",
            "size": file_data.len(),
            "mime": "text/plain",
            "chunk_size": 1024
        }]
    });
    let prepare_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/uploads/chunks/prepare?token={}",
            room_name, token
        ),
        Some(Body::from(prepare_payload.to_string())),
    );

    let prepare_response = app.clone().oneshot(prepare_request).await?;
    assert_eq!(prepare_response.status(), StatusCode::OK);

    let prepare_body = axum::body::to_bytes(prepare_response.into_body(), usize::MAX).await?;
    let prepare_json: serde_json::Value = serde_json::from_slice(&prepare_body)?;
    let upload_token = prepare_json["upload_token"]
        .as_str()
        .expect("upload token")
        .to_string();
    let reservation_id = prepare_json["reservation_id"]
        .as_str()
        .expect("reservation id")
        .to_string();

    // 4. 分块上传 - 使用 multipart/form-data 和正确的字段
    let boundary = "----chunked-upload-boundary";
    let chunk_body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"upload_token\"\r\n\
         \r\n\
         {}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk_index\"\r\n\
         \r\n\
         0\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk_size\"\r\n\
         \r\n\
         {}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk_data\"; filename=\"test_file.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{boundary}--\r\n",
        upload_token,
        file_data.len(),
        file_data
    );

    let upload_request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/{}/uploads/chunks?token={}", room_name, token))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(chunk_body))?;

    let upload_response = app.clone().oneshot(upload_request).await?;
    assert_eq!(upload_response.status(), StatusCode::OK);

    let upload_body = axum::body::to_bytes(upload_response.into_body(), usize::MAX).await?;
    let upload_json: serde_json::Value = serde_json::from_slice(&upload_body)?;
    assert_eq!(upload_json["chunk_index"], 0);
    assert_eq!(upload_json["chunk_size"], file_data.len());
    assert!(upload_json["chunk_hash"].is_string());

    // 5. 获取上传状态
    let status_request = create_http_request(
        Method::GET,
        &format!(
            "/api/v1/rooms/{}/uploads/chunks/status?token={}&upload_token={}",
            room_name, token, upload_token
        ),
        None,
    );

    let status_response = app.clone().oneshot(status_request).await?;
    assert_eq!(status_response.status(), StatusCode::OK);

    // 6. 完成文件合并
    let mut hasher = Sha256::new();
    hasher.update(file_data.as_bytes());
    let final_hash = hex::encode(hasher.finalize());
    let complete_payload = json!({
        "reservation_id": reservation_id,
        "final_hash": final_hash
    });
    let complete_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/uploads/chunks/complete?token={}", room_name, token),
        Some(Body::from(complete_payload.to_string())),
    );

    let complete_response = app.clone().oneshot(complete_request).await?;
    assert_eq!(complete_response.status(), StatusCode::OK);
    let complete_body = axum::body::to_bytes(complete_response.into_body(), usize::MAX).await?;
    let complete_json: serde_json::Value = serde_json::from_slice(&complete_body)?;
    assert_eq!(complete_json["reservation_id"], reservation_id);
    assert_eq!(
        complete_json["merged_files"][0]["file_name"],
        "test_file.txt"
    );
    assert_eq!(complete_json["merged_files"][0]["file_hash"], final_hash);
    let content_id = complete_json["merged_files"][0]["content_id"]
        .as_i64()
        .expect("content id");

    let stored_path: String =
        sqlx::query_scalar("SELECT path FROM room_contents WHERE id = $1 AND room_id = (SELECT id FROM rooms WHERE name = $2)")
            .bind(content_id)
            .bind(room_name)
            .fetch_one(_pool.as_ref())
            .await?;
    assert!(
        stored_path.starts_with(std::env::temp_dir().to_string_lossy().as_ref()),
        "stored path should honor configured storage root, got {stored_path}"
    );
    assert!(
        tokio::fs::try_exists(&stored_path).await?,
        "merged file should exist at configured storage root"
    );

    Ok(())
}

/// 测试分块上传错误处理
#[tokio::test]
async fn test_chunked_upload_error_handling() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "error_test_room";

    // 创建房间
    let create_request = create_room_request(room_name, Some("error123"));

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(json!({ "password": "error123" }).to_string())),
    );
    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token = issue_json["token"].as_str().expect("token");

    // 测试无效的上传令牌
    let boundary = "----error-boundary";
    let invalid_chunk_data = "test data";
    let chunk_body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"upload_token\"\r\n\
         \r\n\
         invalid_token\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk_index\"\r\n\
         \r\n\
         0\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk_size\"\r\n\
         \r\n\
         {}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"chunk_data\"; filename=\"error.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{boundary}--\r\n",
        invalid_chunk_data.len(),
        invalid_chunk_data
    );

    let upload_request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/{}/uploads/chunks?token={}", room_name, token))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(chunk_body))?;

    let upload_response = app.clone().oneshot(upload_request).await?;
    assert_eq!(upload_response.status(), StatusCode::NOT_FOUND);

    // 测试缺少认证 token 的预检请求
    let missing_token_request = create_http_request(
        Method::POST,
        "/api/v1/rooms/nonexistent_room/uploads/chunks/prepare",
        Some(Body::from(
            json!({
                "files": [{
                    "name": "test.txt",
                    "size": 10,
                    "mime": "text/plain"
                }]
            })
            .to_string(),
        )),
    );

    let missing_token_response = app.clone().oneshot(missing_token_request).await?;
    assert_eq!(missing_token_response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

/// 测试分块上传状态查询
#[tokio::test]
async fn test_chunked_upload_status() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "status_test_room";

    // 创建房间
    let create_request = create_room_request(room_name, Some("status123"));

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 发放访问令牌
    let issue_payload = json!({ "password": "status123" });
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

    // 预检上传
    let prepare_payload = json!({
        "files": [{
            "name": "status_test.txt",
            "size": 20,
            "mime": "text/plain"
        }]
    });
    let prepare_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/uploads/chunks/prepare?token={}",
            room_name, token
        ),
        Some(Body::from(prepare_payload.to_string())),
    );

    let prepare_response = app.clone().oneshot(prepare_request).await?;
    assert_eq!(prepare_response.status(), StatusCode::OK);

    let prepare_body = axum::body::to_bytes(prepare_response.into_body(), usize::MAX).await?;
    let prepare_json: serde_json::Value = serde_json::from_slice(&prepare_body)?;
    let upload_token = prepare_json["upload_token"].as_str().unwrap().to_string();

    // 查询上传状态
    let status_request = create_http_request(
        Method::GET,
        &format!(
            "/api/v1/rooms/{}/uploads/chunks/status?token={}&upload_token={}",
            room_name, token, upload_token
        ),
        None,
    );

    let status_response = app.clone().oneshot(status_request).await?;
    assert_eq!(status_response.status(), StatusCode::OK);

    Ok(())
}

/// 测试分块上传预检错误处理
#[tokio::test]
async fn test_chunked_upload_prepare_errors() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "prepare_error_room";

    // 创建房间
    let create_request = create_room_request(room_name, None);

    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);

    // 发放访问令牌
    let issue_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}/tokens", room_name),
        Some(Body::from(json!({}).to_string())),
    );

    let issue_response = app.clone().oneshot(issue_request).await?;
    assert_eq!(issue_response.status(), StatusCode::OK);

    let issue_body = axum::body::to_bytes(issue_response.into_body(), usize::MAX).await?;
    let issue_json: serde_json::Value = serde_json::from_slice(&issue_body)?;
    let token = issue_json["token"].as_str().expect("token string");

    // 测试空文件列表
    let empty_files_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/uploads/chunks/prepare?token={}",
            room_name, token
        ),
        Some(Body::from(json!({ "files": [] }).to_string())),
    );

    let empty_files_response = app.clone().oneshot(empty_files_request).await?;
    assert_eq!(empty_files_response.status(), StatusCode::BAD_REQUEST);

    // 测试无效文件大小
    let invalid_size_request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/uploads/chunks/prepare?token={}",
            room_name, token
        ),
        Some(Body::from(
            json!({
                "files": [{
                    "name": "invalid.txt",
                    "size": 0,
                    "mime": "text/plain"
                }]
            })
            .to_string(),
        )),
    );

    let invalid_size_response = app.clone().oneshot(invalid_size_request).await?;
    assert_eq!(invalid_size_response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}
