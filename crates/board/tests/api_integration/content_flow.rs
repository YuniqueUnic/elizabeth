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
    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let create_json: serde_json::Value = serde_json::from_slice(&create_body)?;
    let admin_token = create_json["token"].as_str().expect("admin token");
    assert_eq!(create_json["claims"]["role"], "admin");

    // 签发 editor token using the creator identity
    let issue_payload = json!({ "token": admin_token, "role": "editor" });
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

// ---------------------------------------------------------------------------
// 房间级上传文件类型策略（issue #199）
// ---------------------------------------------------------------------------

async fn create_room_with_admin_token(app: axum::Router, room_name: &str) -> Result<String> {
    let create_request = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{}", room_name),
        Some(Body::from(json!({}).to_string())),
    );
    let create_response = app.clone().oneshot(create_request).await?;
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let create_json: serde_json::Value = serde_json::from_slice(&create_body)?;
    Ok(create_json["token"]
        .as_str()
        .expect("admin token")
        .to_string())
}

async fn put_room_settings(
    app: axum::Router,
    room_name: &str,
    token: &str,
    payload: serde_json::Value,
) -> Result<(StatusCode, serde_json::Value)> {
    let request = create_http_request(
        Method::PUT,
        &format!("/api/v1/rooms/{}/settings?token={}", room_name, token),
        Some(Body::from(payload.to_string())),
    );
    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;
    Ok((status, json))
}

async fn prepare_file_upload(
    app: axum::Router,
    room_name: &str,
    token: &str,
    file_name: &str,
    size: i64,
) -> Result<(StatusCode, serde_json::Value)> {
    let payload = json!({
        "files": [{ "name": file_name, "size": size, "mime": "application/octet-stream" }]
    });
    let request = create_http_request(
        Method::POST,
        &format!(
            "/api/v1/rooms/{}/contents/prepare?token={}",
            room_name, token
        ),
        Some(Body::from(payload.to_string())),
    );
    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;
    Ok((status, json))
}

#[tokio::test]
async fn test_upload_file_type_allow_policy() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "upload_type_allow_room";
    let token = create_room_with_admin_token(app.clone(), room_name).await?;

    // happy path：设置 allow 策略并回显规范化结果
    let (status, settings_json) = put_room_settings(
        app.clone(),
        room_name,
        &token,
        json!({ "upload_file_type": { "mode": "allow", "extensions": [".PNG", "pdf"] } }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(settings_json["upload_file_type"]["mode"], "allow");
    assert_eq!(
        settings_json["upload_file_type"]["extensions"],
        json!(["png", "pdf"])
    );

    // unhappy：被拒类型与无扩展名文件
    let (status, body) = prepare_file_upload(app.clone(), room_name, &token, "evil.exe", 4).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"]["message"],
        "Validation error: File type not allowed by room policy: evil.exe"
    );

    let (status, _) = prepare_file_upload(app.clone(), room_name, &token, "noext", 4).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // happy：白名单内文件可走完整上传链路
    let (status, prepare_json) =
        prepare_file_upload(app.clone(), room_name, &token, "photo.png", 9).await?;
    assert_eq!(status, StatusCode::OK);
    let reservation_id = prepare_json["reservation_id"]
        .as_i64()
        .expect("reservation id");

    let boundary = "----allow-policy-boundary";
    let file_body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"photo.png\"\r\n\
         Content-Type: image/png\r\n\r\n\
         png-data!\r\n\
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

    Ok(())
}

#[tokio::test]
async fn test_upload_file_type_deny_policy_with_normalization() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "upload_type_deny_room";
    let token = create_room_with_admin_token(app.clone(), room_name).await?;

    // deny 策略：输入带点、大写、空白，服务端统一规范化
    let (status, settings_json) = put_room_settings(
        app.clone(),
        room_name,
        &token,
        json!({ "upload_file_type": { "mode": "deny", "extensions": [" .EXE ", "Sh"] } }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        settings_json["upload_file_type"]["extensions"],
        json!(["exe", "sh"])
    );

    let (status, body) = prepare_file_upload(app.clone(), room_name, &token, "run.EXE", 4).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"]["message"],
        "Validation error: File type not allowed by room policy: run.EXE"
    );

    let (status, _) = prepare_file_upload(app.clone(), room_name, &token, "run.sh", 4).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // happy：非 deny 列表内正常放行
    let (status, _) = prepare_file_upload(app.clone(), room_name, &token, "notes.txt", 4).await?;
    assert_eq!(status, StatusCode::OK);

    // GET 房间详情应携带策略（RoomView）
    let get_request = create_http_request(
        Method::GET,
        &format!("/api/v1/rooms/{}?token={}", room_name, token),
        None,
    );
    let get_response = app.clone().oneshot(get_request).await?;
    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body = axum::body::to_bytes(get_response.into_body(), usize::MAX).await?;
    let get_json: serde_json::Value = serde_json::from_slice(&get_body)?;
    assert_eq!(get_json["upload_file_type"]["mode"], "deny");
    assert_eq!(
        get_json["upload_file_type"]["extensions"],
        json!(["exe", "sh"])
    );

    Ok(())
}

#[tokio::test]
async fn test_upload_file_type_settings_validation() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "upload_type_validation_room";
    let token = create_room_with_admin_token(app.clone(), room_name).await?;

    // unhappy：allow/deny 模式要求列表非空
    let (status, body) = put_room_settings(
        app.clone(),
        room_name,
        &token,
        json!({ "upload_file_type": { "mode": "allow", "extensions": [] } }),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("must not be empty for allow or deny mode")
    );

    // unhappy：非法字符的扩展名
    let (status, body) = put_room_settings(
        app.clone(),
        room_name,
        &token,
        json!({ "upload_file_type": { "mode": "deny", "extensions": ["a$b"] } }),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"]["message"],
        "Validation error: Invalid file type extension: a$b"
    );

    // happy：any 模式忽略（清空）扩展名列表
    let (status, settings_json) = put_room_settings(
        app.clone(),
        room_name,
        &token,
        json!({ "upload_file_type": { "mode": "any", "extensions": ["exe"] } }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(settings_json["upload_file_type"]["mode"], "any");
    assert_eq!(settings_json["upload_file_type"]["extensions"], json!([]));

    Ok(())
}
