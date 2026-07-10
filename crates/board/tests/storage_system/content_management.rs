use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{create_test_app, http::create_request};

use super::{create_room_and_issue_session, response_json};

#[tokio::test]
async fn test_content_management_text() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "text_content_test_room";
    let session = create_room_and_issue_session(&app, room_name, Some("text123")).await?;

    let create = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/messages?token={}", session.token),
            Some(Body::from(
                json!({ "text": "Hello, this is test content!" }).to_string(),
            )),
        ))
        .await?;
    assert_eq!(create.status(), StatusCode::OK);
    let created = response_json(create).await?;
    assert_eq!(created["message"]["text"], "Hello, this is test content!");

    let list = app
        .oneshot(create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/messages?token={}", session.token),
            None,
        ))
        .await?;
    assert_eq!(list.status(), StatusCode::OK);
    let page = response_json(list).await?;
    assert_eq!(page["items"].as_array().expect("message items").len(), 1);
    assert_eq!(page["items"][0]["text"], "Hello, this is test content!");
    Ok(())
}

#[tokio::test]
async fn test_content_management_urls() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "url_content_test_room";
    let session = create_room_and_issue_session(&app, room_name, None).await?;

    let create = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!(
                "/api/v1/rooms/{room_name}/contents/url?token={}",
                session.token
            ),
            Some(Body::from(
                json!({
                    "url": "https://example.com/image.png",
                    "name": "Example image",
                    "description": "remote asset"
                })
                .to_string(),
            )),
        ))
        .await?;
    assert_eq!(create.status(), StatusCode::OK);
    let created = response_json(create).await?;
    assert_eq!(created["created"]["url"], "https://example.com/image.png");
    assert_eq!(created["created"]["file_name"], "Example image");

    let list = app
        .oneshot(create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/contents?token={}", session.token),
            None,
        ))
        .await?;
    assert_eq!(list.status(), StatusCode::OK);
    let contents = response_json(list).await?;
    assert_eq!(contents.as_array().expect("room contents").len(), 1);
    assert_eq!(contents[0]["url"], "https://example.com/image.png");
    Ok(())
}

#[tokio::test]
async fn test_content_management_mixed() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "mixed_content_test_room";
    let session = create_room_and_issue_session(&app, room_name, None).await?;

    for (path, payload) in [
        (
            "messages",
            json!({ "text": "A message in the mixed room", "sequence_number": 1 }),
        ),
        (
            "contents/url",
            json!({
                "url": "https://example.com/docs",
                "name": "Documentation",
                "description": "A link in the mixed room"
            }),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(create_request(
                Method::POST,
                &format!("/api/v1/rooms/{room_name}/{path}?token={}", session.token),
                Some(Body::from(payload.to_string())),
            ))
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
    }

    let list = app
        .oneshot(create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/contents?token={}", session.token),
            None,
        ))
        .await?;
    assert_eq!(list.status(), StatusCode::OK);
    let contents = response_json(list).await?;
    let contents = contents.as_array().expect("mixed room contents");
    assert_eq!(contents.len(), 2);
    assert!(
        contents
            .iter()
            .any(|item| item["text"] == "A message in the mixed room")
    );
    assert!(
        contents
            .iter()
            .any(|item| item["url"] == "https://example.com/docs")
    );
    Ok(())
}

#[tokio::test]
async fn test_content_management_errors() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "error_content_test_room";
    let session = create_room_and_issue_session(&app, room_name, None).await?;

    let missing_token = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/messages"),
            Some(Body::from(json!({ "text": "must fail" }).to_string())),
        ))
        .await?;
    assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

    let invalid_token = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/messages?token=invalid_token"),
            Some(Body::from(json!({ "text": "must fail" }).to_string())),
        ))
        .await?;
    assert_eq!(invalid_token.status(), StatusCode::UNAUTHORIZED);

    let invalid_url = app
        .oneshot(create_request(
            Method::POST,
            &format!(
                "/api/v1/rooms/{room_name}/contents/url?token={}",
                session.token
            ),
            Some(Body::from(
                json!({ "url": "not-a-url", "name": "invalid" }).to_string(),
            )),
        ))
        .await?;
    assert_eq!(invalid_url.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
