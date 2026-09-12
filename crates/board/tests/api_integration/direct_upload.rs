//! 单命令 PUT 上传（issue #201）：`PUT /api/v1/rooms/{name}/files/{filename}`。
//! 覆盖 happy path（上传 → download_url → 下载回读）与各 unhappy path
//! （缺鉴权、不安全文件名、Content-Length 不符、容量超限、文件类型策略）。

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::common::create_test_app;

async fn body_json(response: axum::response::Response) -> Result<Value> {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

async fn create_room(app: &axum::Router, name: &str) -> Result<String> {
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/{name}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))?;
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await?;
    Ok(json["token"].as_str().expect("admin token").to_string())
}

fn put_request(
    room: &str,
    filename: &str,
    token: Option<&str>,
    content_length: Option<i64>,
    body: &'static [u8],
) -> Result<Request<Body>> {
    let mut builder = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/rooms/{room}/files/{filename}"));
    if let Some(length) = content_length {
        builder = builder.header("content-length", length.to_string());
    }
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    Ok(builder.body(Body::from(body))?)
}

async fn put_upload(
    app: &axum::Router,
    room: &str,
    filename: &str,
    token: Option<&str>,
    body: &'static [u8],
) -> Result<axum::response::Response> {
    app.clone()
        .oneshot(put_request(
            room,
            filename,
            token,
            Some(body.len() as i64),
            body,
        )?)
        .await
        .map_err(Into::into)
}

async fn room_current_size(app: &axum::Router, room: &str, token: &str) -> Result<i64> {
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/rooms/{room}?token={token}"))
        .body(Body::empty())?;
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(body_json(response).await?["current_size"]
        .as_i64()
        .expect("current_size"))
}

#[tokio::test]
async fn test_put_upload_returns_download_url_and_downloads_back() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let token = create_room(&app, "direct-upload").await?;
    let payload: &'static [u8] = b"hello elizabeth cli";

    let response = put_upload(&app, "direct-upload", "report.txt", Some(&token), payload).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await?;

    assert_eq!(json["current_size"], json!(payload.len() as i64));
    let uploaded = json["uploaded"].as_array().expect("uploaded array");
    assert_eq!(uploaded.len(), 1);
    assert_eq!(uploaded[0]["file_name"], json!("report.txt"));
    let content_id = uploaded[0]["id"].as_i64().expect("content id");
    let download_url = uploaded[0]["download_url"].as_str().expect("download url");
    assert_eq!(
        download_url,
        &format!("/api/v1/contents/{content_id}"),
        "download_url 必须是可直接拼接凭据的相对路径"
    );

    // download_url 不携带凭据：不带 token 直接下载必须被拒绝。
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/contents/{content_id}"))
        .body(Body::empty())?;
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 附加房间 token 后按 download_url 下载，字节与上传内容一致。
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/contents/{content_id}?token={token}"))
        .body(Body::empty())?;
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    assert_eq!(&bytes[..], payload);

    Ok(())
}

#[tokio::test]
async fn test_put_upload_requires_room_token() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    create_room(&app, "no-auth-room").await?;

    let response = put_upload(&app, "no-auth-room", "a.txt", None, b"data").await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(response).await?;
    assert_eq!(json["error"]["code"], json!("AUTHENTICATION_FAILED"));

    Ok(())
}

#[tokio::test]
async fn test_put_upload_rejects_unsafe_file_names() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let token = create_room(&app, "unsafe-name-room").await?;

    for filename in ["..", "a%2Fb.txt", "%2E%2E%2Fescape.txt"] {
        let response = put_upload(&app, "unsafe-name-room", filename, Some(&token), b"x").await?;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "filename: {filename}"
        );
        let json = body_json(response).await?;
        assert_eq!(json["error"]["code"], json!("VALIDATION_ERROR"));
        assert!(
            json["error"]["message"]
                .as_str()
                .unwrap()
                .starts_with("Validation error: Invalid file name"),
            "unexpected message: {json}"
        );
    }

    assert_eq!(
        room_current_size(&app, "unsafe-name-room", &token).await?,
        0
    );
    Ok(())
}

#[tokio::test]
async fn test_put_upload_rejects_missing_and_mismatched_content_length() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let token = create_room(&app, "length-room").await?;

    // 缺少 Content-Length 直接拒绝。
    let request = put_request("length-room", "a.txt", Some(&token), None, b"data")?;
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response).await?;
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Content-Length"),
        "unexpected message: {json}"
    );

    // 声明 4 字节但发送 9 字节：拒绝、清理落盘文件并释放预留额度。
    let request = put_request(
        "length-room",
        "mismatch.bin",
        Some(&token),
        Some(4),
        b"123456789",
    )?;
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(room_current_size(&app, "length-room", &token).await?, 0);

    // 额度已释放：同样的 token 仍可正常上传。
    let response = put_upload(&app, "length-room", "ok.txt", Some(&token), b"fine").await?;
    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_put_upload_enforces_room_capacity() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let token = create_room(&app, "capacity-room").await?;

    let settings = Request::builder()
        .method(Method::PUT)
        .uri(format!(
            "/api/v1/rooms/capacity-room/settings?token={token}"
        ))
        .header("content-type", "application/json")
        .body(Body::from(json!({ "max_size": 8 }).to_string()))?;
    let response = app.clone().oneshot(settings).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = put_upload(
        &app,
        "capacity-room",
        "big.bin",
        Some(&token),
        b"0123456789",
    )
    .await?;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let json = body_json(response).await?;
    assert_eq!(json["error"]["code"], json!("PAYLOAD_TOO_LARGE"));
    assert_eq!(room_current_size(&app, "capacity-room", &token).await?, 0);

    Ok(())
}

#[tokio::test]
async fn test_put_upload_enforces_room_file_type_policy() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let token = create_room(&app, "policy-room").await?;

    let settings = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/rooms/policy-room/settings?token={token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "upload_file_type": { "mode": "allow", "extensions": ["pdf", "png"] } })
                .to_string(),
        ))?;
    let response = app.clone().oneshot(settings).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = put_upload(&app, "policy-room", "evil.exe", Some(&token), b"MZ").await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response).await?;
    assert_eq!(json["error"]["code"], json!("VALIDATION_ERROR"));
    assert_eq!(
        json["error"]["message"],
        json!("Validation error: File type not allowed by room policy: evil.exe")
    );

    let response = put_upload(&app, "policy-room", "doc.pdf", Some(&token), b"%PDF-1.4").await?;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await?;
    assert_eq!(
        json["uploaded"][0]["download_url"],
        json!(format!("/api/v1/contents/{}", json["uploaded"][0]["id"]))
    );

    Ok(())
}
