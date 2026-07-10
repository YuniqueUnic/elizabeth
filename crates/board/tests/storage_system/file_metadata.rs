use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{create_test_app, http::create_request};

use super::{
    create_room, create_room_and_issue_session, issue_session, response_json, upload_file,
};

#[tokio::test]
async fn test_file_metadata() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "file_metadata_test_room";
    let session = create_room_and_issue_session(&app, room_name, None).await?;
    upload_file(
        &app,
        room_name,
        &session.token,
        "metadata.txt",
        "text/plain",
        b"metadata payload",
    )
    .await?;

    let response = app
        .oneshot(create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/contents?token={}", session.token),
            None,
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let contents = response_json(response).await?;
    let item = &contents[0];
    assert_eq!(item["file_name"], "metadata.txt");
    assert_eq!(item["size"], 16);
    assert_eq!(item["mime_type"], "text/plain");
    assert!(
        item.get("path").is_none(),
        "storage paths must not leak through the API"
    );
    Ok(())
}

#[tokio::test]
async fn test_storage_cleanup() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let room_name = "storage_cleanup_test_room";
    let session = create_room_and_issue_session(&app, room_name, None).await?;
    let uploaded = upload_file(
        &app,
        room_name,
        &session.token,
        "cleanup.txt",
        "text/plain",
        b"remove me",
    )
    .await?;
    let content_id = uploaded["uploaded"][0]["id"].as_i64().expect("content id");
    let path: String = sqlx::query_scalar("SELECT path FROM room_contents WHERE id = $1")
        .bind(content_id)
        .fetch_one(pool.as_ref())
        .await?;
    assert!(tokio::fs::try_exists(&path).await?);

    let delete = app
        .clone()
        .oneshot(create_request(
            Method::DELETE,
            &format!("/api/v1/rooms/{room_name}/contents?token={}", session.token),
            Some(Body::from(json!({ "ids": [content_id] }).to_string())),
        ))
        .await?;
    assert_eq!(delete.status(), StatusCode::OK);
    let deleted = response_json(delete).await?;
    assert_eq!(deleted["deleted"], json!([content_id]));
    assert!(!tokio::fs::try_exists(&path).await?);

    let list = app
        .oneshot(create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/contents?token={}", session.token),
            None,
        ))
        .await?;
    assert_eq!(response_json(list).await?, json!([]));
    Ok(())
}

#[tokio::test]
async fn test_storage_permissions() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "storage_perms_test_room";
    create_room(&app, room_name, Some("storage123")).await?;

    let missing_token = app
        .clone()
        .oneshot(create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/contents"),
            None,
        ))
        .await?;
    assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

    let wrong_password = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/tokens"),
            Some(Body::from(json!({ "password": "wrong" }).to_string())),
        ))
        .await?;
    assert_eq!(wrong_password.status(), StatusCode::UNAUTHORIZED);

    let session = issue_session(&app, room_name, Some("storage123")).await?;
    let authorized = app
        .oneshot(create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/contents?token={}", session.token),
            None,
        ))
        .await?;
    assert_eq!(authorized.status(), StatusCode::OK);
    Ok(())
}
