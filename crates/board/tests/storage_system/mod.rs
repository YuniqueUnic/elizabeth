mod content_management;
mod file_metadata;
mod upload_and_paths;

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::common::http::create_request;

pub(crate) struct IssuedSession {
    pub token: String,
    pub jti: String,
}

pub(crate) async fn create_room(
    app: &Router,
    room_name: &str,
    password: Option<&str>,
) -> Result<()> {
    let response = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}"),
            Some(Body::from(
                password
                    .map(|password| json!({ "password": password }))
                    .unwrap_or_else(|| json!({}))
                    .to_string(),
            )),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}

pub(crate) async fn issue_session(
    app: &Router,
    room_name: &str,
    password: Option<&str>,
) -> Result<IssuedSession> {
    let response = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/tokens"),
            Some(Body::from(
                password
                    .map(|password| json!({ "password": password }))
                    .unwrap_or_else(|| json!({}))
                    .to_string(),
            )),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let value = response_json(response).await?;
    Ok(IssuedSession {
        token: value["token"].as_str().expect("access token").to_string(),
        jti: value["claims"]["jti"]
            .as_str()
            .expect("access token jti")
            .to_string(),
    })
}

pub(crate) async fn create_room_and_issue_session(
    app: &Router,
    room_name: &str,
    password: Option<&str>,
) -> Result<IssuedSession> {
    create_room(app, room_name, password).await?;
    issue_session(app, room_name, password).await
}

pub(crate) async fn upload_file(
    app: &Router,
    room_name: &str,
    token: &str,
    file_name: &str,
    mime: &str,
    contents: &[u8],
) -> Result<Value> {
    let prepare = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/contents/prepare?token={token}"),
            Some(Body::from(
                json!({
                    "files": [{
                        "name": file_name,
                        "size": contents.len(),
                        "mime": mime
                    }]
                })
                .to_string(),
            )),
        ))
        .await?;
    assert_eq!(prepare.status(), StatusCode::OK);
    let reservation_id = response_json(prepare).await?["reservation_id"]
        .as_i64()
        .expect("reservation id");

    let boundary = "----elizabeth-storage-system-boundary";
    let mut multipart = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: {mime}\r\n\r\n"
    )
    .into_bytes();
    multipart.extend_from_slice(contents);
    multipart.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let upload = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/rooms/{room_name}/contents?token={token}&reservation_id={reservation_id}"
                ))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(multipart))?,
        )
        .await?;
    assert_eq!(upload.status(), StatusCode::OK);
    response_json(upload).await
}

pub(crate) async fn response_json(response: axum::response::Response) -> Result<Value> {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&body)?)
}
