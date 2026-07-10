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
async fn test_room_token_and_content_flow() -> Result<()> {
    let (app, _pool) = create_test_app().await?;

    let room_name = "content_test_room";

    // 创建房间
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}", room_name),
        Some(Body::from(json!({ "password": "secret" }).to_string())),
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
    let upload_body = axum::body::to_bytes(upload_response.into_body(), usize::MAX).await?;
    let upload_json: serde_json::Value = serde_json::from_slice(&upload_body)?;
    let content_id = upload_json["uploaded"][0]["id"]
        .as_i64()
        .expect("uploaded content id");

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
    assert!(
        items
            .iter()
            .any(|item| item["id"].as_i64() == Some(content_id)),
        "uploaded content should be visible in list response"
    );

    // 下载文件
    let download_request = create_http_request(
        Method::GET,
        &format!("/api/v1/contents/{}?token={}", content_id, token),
        None,
    );
    let download_response = app.clone().oneshot(download_request).await?;
    assert_eq!(download_response.status(), StatusCode::OK);
    let download_body = axum::body::to_bytes(download_response.into_body(), usize::MAX).await?;
    assert_eq!(download_body.as_ref(), b"hello world");

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
