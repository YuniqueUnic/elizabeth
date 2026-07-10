use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{create_test_app, http::create_request};

use super::{create_room_and_issue_session, issue_session, response_json, upload_file};

#[tokio::test]
async fn test_upload_reservation() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let room_name = "upload_reservation_test_room";
    let session = create_room_and_issue_session(&app, room_name, None).await?;

    let reserve = app
        .oneshot(create_request(
            Method::POST,
            &format!(
                "/api/v1/rooms/{room_name}/contents/prepare?token={}",
                session.token
            ),
            Some(Body::from(
                json!({
                    "files": [{
                        "name": "test_file.txt",
                        "size": 1024,
                        "mime": "text/plain"
                    }]
                })
                .to_string(),
            )),
        ))
        .await?;
    assert_eq!(reserve.status(), StatusCode::OK);
    let reservation_id = response_json(reserve).await?["reservation_id"]
        .as_i64()
        .expect("reservation id");
    let (owner_jti, reserved_size): (String, i64) = sqlx::query_as(
        "SELECT owner_token_jti, reserved_size FROM room_upload_reservations WHERE id = $1",
    )
    .bind(reservation_id)
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(owner_jti, session.jti);
    assert_eq!(reserved_size, 1024);
    Ok(())
}

#[tokio::test]
async fn test_chunk_merge() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "chunk_owner_test_room";
    let owner = create_room_and_issue_session(&app, room_name, None).await?;
    let other = issue_session(&app, room_name, None).await?;

    let prepare = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!(
                "/api/v1/rooms/{room_name}/uploads/chunks/prepare?token={}",
                owner.token
            ),
            Some(Body::from(
                json!({
                    "files": [{
                        "name": "owned.txt",
                        "size": 5,
                        "mime": "text/plain",
                        "chunk_size": 5
                    }]
                })
                .to_string(),
            )),
        ))
        .await?;
    assert_eq!(prepare.status(), StatusCode::OK);
    let upload_token = response_json(prepare).await?["upload_token"]
        .as_str()
        .expect("upload token")
        .to_string();

    let wrong_owner = app
        .clone()
        .oneshot(create_request(
            Method::GET,
            &format!(
                "/api/v1/rooms/{room_name}/uploads/chunks/status?token={}&upload_token={upload_token}",
                other.token
            ),
            None,
        ))
        .await?;
    assert_eq!(wrong_owner.status(), StatusCode::FORBIDDEN);

    let owner_status = app
        .oneshot(create_request(
            Method::GET,
            &format!(
                "/api/v1/rooms/{room_name}/uploads/chunks/status?token={}&upload_token={upload_token}",
                owner.token
            ),
            None,
        ))
        .await?;
    assert_eq!(owner_status.status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_storage_paths() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let room_name = "storage_path_test_room";
    let session = create_room_and_issue_session(&app, room_name, None).await?;
    let uploaded = upload_file(
        &app,
        room_name,
        &session.token,
        "../unsafe.txt",
        "text/plain",
        b"safe",
    )
    .await?;
    let content_id = uploaded["uploaded"][0]["id"].as_i64().expect("content id");
    let path: String = sqlx::query_scalar("SELECT path FROM room_contents WHERE id = $1")
        .bind(content_id)
        .fetch_one(pool.as_ref())
        .await?;
    let storage_root = std::env::temp_dir();
    assert!(path.starts_with(storage_root.to_string_lossy().as_ref()));
    assert!(!path.ends_with("../unsafe.txt"));
    assert!(tokio::fs::try_exists(path).await?);
    Ok(())
}
