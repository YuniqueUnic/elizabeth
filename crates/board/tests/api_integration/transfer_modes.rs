//! 传输策略（issue #198）：proxy（默认，桶地址不出现在任何响应）与
//! presigned（鉴权后签发短时效直传/直下 URL）两种模式的行为边界。

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

use crate::common::mocks::storage::InMemoryStorageBackend;
use crate::common::{
    create_test_app, create_test_app_with_config,
    http::{assert_status, create_request as create_http_request},
};

async fn body_json(response: axum::response::Response) -> Result<Value> {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

async fn create_room(app: &axum::Router, name: &str) -> Result<String> {
    let request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{name}"),
        Some(Body::from(json!({}).to_string())),
    );
    let response = app.clone().oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(body_json(response).await?["token"]
        .as_str()
        .expect("admin token")
        .to_string())
}

#[tokio::test]
async fn test_proxy_mode_never_exposes_presign_urls() -> Result<()> {
    let storage = Arc::new(InMemoryStorageBackend::new("s3.internal.test"));
    let (app, _pool) = create_test_app_with_config(Some(storage.clone()), |_| {}).await?;
    let token = create_room(&app, "proxy-room").await?;

    // 两阶段 prepare：proxy 模式响应不得出现 presigned_uploads
    let prepare = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/proxy-room/contents/prepare?token={token}"),
        Some(Body::from(
            json!({"files": [{"name": "a.txt", "size": 5, "mime": "text/plain"}]}).to_string(),
        )),
    );
    let response = app.clone().oneshot(prepare).await?;
    assert_status(&response, StatusCode::OK);
    let prepared = body_json(response).await?;
    assert!(
        prepared.get("presigned_uploads").is_none(),
        "proxy 模式不应返回直传 URL: {prepared}"
    );
    let reservation_id = prepared["reservation_id"].as_i64().expect("reservation id");

    // multipart 直传服务器
    let boundary = "----transfer-proxy-boundary";
    let mut multipart = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\nContent-Type: text/plain\r\n\r\n"
    )
    .into_bytes();
    multipart.extend_from_slice(b"hello");
    multipart.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let upload = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/rooms/proxy-room/contents?token={token}&reservation_id={reservation_id}"
        ))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(multipart))?;
    let response = app.clone().oneshot(upload).await?;
    assert_status(&response, StatusCode::OK);
    let uploaded = body_json(response).await?;
    let content_id = uploaded["uploaded"][0]["id"].as_i64().expect("content id");

    // 下载为代理流：内容直出，无 Location 跳转
    let download = create_http_request(
        Method::GET,
        &format!("/api/v1/contents/{content_id}?token={token}"),
        None,
    );
    let response = app.clone().oneshot(download).await?;
    assert_status(&response, StatusCode::OK);
    assert!(
        response.headers().get("location").is_none(),
        "proxy 模式下载不应重定向"
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    assert_eq!(&bytes[..], b"hello");

    // 存储后端里的内容也不应通过任何 API 泄漏直链
    assert!(
        !uploaded.to_string().contains("s3.internal.test"),
        "上传响应不得包含存储地址: {uploaded}"
    );
    Ok(())
}

#[tokio::test]
async fn test_presigned_upload_download_direct_flow() -> Result<()> {
    let storage = Arc::new(InMemoryStorageBackend::new("cdn.example.com"));
    let (app, _pool) = create_test_app_with_config(Some(storage.clone()), |config| {
        config.storage.transfer = board::config::TransferMode::Presigned;
    })
    .await?;
    let token = create_room(&app, "presigned-room").await?;

    // prepare：鉴权与配额校验后返回逐文件直传 URL
    let prepare = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/presigned-room/contents/prepare?token={token}"),
        Some(Body::from(
            json!({"files": [{"name": "report.pdf", "size": 8, "mime": "application/pdf"}]})
                .to_string(),
        )),
    );
    let response = app.clone().oneshot(prepare).await?;
    assert_status(&response, StatusCode::OK);
    let prepared = body_json(response).await?;
    let presigned = prepared["presigned_uploads"]
        .as_array()
        .expect("presigned uploads in presigned mode");
    assert_eq!(presigned.len(), 1);
    assert_eq!(presigned[0]["method"], json!("PUT"));
    assert_eq!(presigned[0]["file_name"], json!("report.pdf"));
    let url = presigned[0]["url"].as_str().expect("presign url");
    assert!(
        url.starts_with("https://cdn.example.com/"),
        "自定义 base URL 应生效: {url}"
    );
    assert!(
        url.contains("X-Amz-Expires=300"),
        "签名 URL 必须携带过期时间: {url}"
    );
    let key = url
        .split("https://cdn.example.com/")
        .nth(1)
        .and_then(|rest| rest.split('?').next())
        .expect("key in presign url")
        .to_string();
    assert!(
        key.split('/')
            .next()
            .unwrap()
            .chars()
            .all(|c| c.is_ascii_digit()),
        "key 必须以数字房间 id 为前缀: {key}"
    );
    let reservation_id = prepared["reservation_id"].as_i64().expect("reservation id");

    let commit = |rid: i64| {
        create_http_request(
            Method::POST,
            &format!("/api/v1/rooms/presigned-room/contents/presigned-commit?token={token}"),
            Some(Body::from(json!({ "reservation_id": rid }).to_string())),
        )
    };

    // 未直传就 commit：400（对象缺失）
    let response = app.clone().oneshot(commit(reservation_id)).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 直传大小不符：400，且不产生内容记录
    storage.put_direct(&key, b"short".to_vec());
    let response = app.clone().oneshot(commit(reservation_id)).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let list = create_http_request(
        Method::GET,
        &format!("/api/v1/rooms/presigned-room/contents?token={token}"),
        None,
    );
    let response = app.clone().oneshot(list).await?;
    let items = body_json(response).await?;
    assert_eq!(
        items.as_array().expect("contents").len(),
        0,
        "失败的提交不应留下内容记录"
    );
    Ok(())
}

#[tokio::test]
async fn test_presigned_commit_and_download() -> Result<()> {
    let storage = Arc::new(InMemoryStorageBackend::new("cdn.example.com"));
    let (app, _pool) = create_test_app_with_config(Some(storage.clone()), |config| {
        config.storage.transfer = board::config::TransferMode::Presigned;
    })
    .await?;
    let token = create_room(&app, "presigned-dl").await?;

    let prepare = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/presigned-dl/contents/prepare?token={token}"),
        Some(Body::from(
            json!({"files": [{"name": "data.bin", "size": 4, "mime": "application/octet-stream"}]})
                .to_string(),
        )),
    );
    let response = app.clone().oneshot(prepare).await?;
    let prepared = body_json(response).await?;
    let reservation_id = prepared["reservation_id"].as_i64().expect("reservation id");
    let url = prepared["presigned_uploads"][0]["url"]
        .as_str()
        .expect("url");
    let key = url
        .split("https://cdn.example.com/")
        .nth(1)
        .and_then(|rest| rest.split('?').next())
        .expect("key")
        .to_string();

    // 模拟客户端直传
    storage.put_direct(&key, b"DATA".to_vec());

    // commit：核对大小、落记录、核销预留
    let commit = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/presigned-dl/contents/presigned-commit?token={token}"),
        Some(Body::from(
            json!({ "reservation_id": reservation_id }).to_string(),
        )),
    );
    let response = app.clone().oneshot(commit).await?;
    assert_status(&response, StatusCode::OK);
    let committed = body_json(response).await?;
    let content_id = committed["uploaded"][0]["id"].as_i64().expect("content id");
    assert_eq!(committed["current_size"], json!(4));
    assert!(
        committed["uploaded"][0]["download_url"]
            .as_str()
            .unwrap()
            .ends_with(&format!("/contents/{content_id}"))
    );

    // 下载：302 跳转到短时效直下 URL（自定义域名 + Expire）
    let download = create_http_request(
        Method::GET,
        &format!("/api/v1/contents/{content_id}?token={token}"),
        None,
    );
    let response = app.clone().oneshot(download).await?;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get("location")
        .expect("redirect location")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        location.starts_with("https://cdn.example.com/"),
        "{location}"
    );
    assert!(location.contains("X-Amz-Expires=300"), "{location}");

    // 预留已核销：重复 commit 返回 400
    let commit = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/presigned-dl/contents/presigned-commit?token={token}"),
        Some(Body::from(
            json!({ "reservation_id": reservation_id }).to_string(),
        )),
    );
    let response = app.clone().oneshot(commit).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

#[tokio::test]
async fn test_presigned_issuance_requires_valid_token() -> Result<()> {
    let storage = Arc::new(InMemoryStorageBackend::new("cdn.example.com"));
    let (app, _pool) = create_test_app_with_config(Some(storage.clone()), |config| {
        config.storage.transfer = board::config::TransferMode::Presigned;
    })
    .await?;
    create_room(&app, "presigned-auth").await?;

    // 无 token：prepare 拿不到任何签名 URL
    let prepare = create_http_request(
        Method::POST,
        "/api/v1/rooms/presigned-auth/contents/prepare",
        Some(Body::from(
            json!({"files": [{"name": "x.txt", "size": 1, "mime": "text/plain"}]}).to_string(),
        )),
    );
    let response = app.clone().oneshot(prepare).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await?;
    assert!(body.to_string().find("cdn.example.com").is_none());

    // 伪 token：同样拿不到
    let prepare = create_http_request(
        Method::POST,
        "/api/v1/rooms/presigned-auth/contents/prepare?token=forged-token",
        Some(Body::from(
            json!({"files": [{"name": "x.txt", "size": 1, "mime": "text/plain"}]}).to_string(),
        )),
    );
    let response = app.clone().oneshot(prepare).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // proxy 默认模式不受影响（回归保护）
    let (app, _pool) = create_test_app().await?;
    let token = create_room(&app, "regression-room").await?;
    let prepare = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/regression-room/contents/prepare?token={token}"),
        Some(Body::from(
            json!({"files": [{"name": "x.txt", "size": 1, "mime": "text/plain"}]}).to_string(),
        )),
    );
    let response = app.clone().oneshot(prepare).await?;
    assert_status(&response, StatusCode::OK);

    Ok(())
}
