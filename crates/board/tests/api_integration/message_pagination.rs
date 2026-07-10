use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use board::{
    dto::content::MessagePage,
    models::content::{ContentType, RoomContent},
    repository::{IRoomContentRepository, RoomContentRepository},
};
use chrono::Utc;
use serde_json::{Value, json};

use crate::common::{
    create_test_app,
    http::{assert_json, create_request, send_request},
};

async fn create_room_and_token(app: &axum::Router, room_name: &str) -> Result<(i64, String)> {
    let response = send_request(
        app.clone(),
        create_request(Method::POST, &format!("/api/v1/rooms/{room_name}"), None),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let room: Value = assert_json(response).await?;
    let room_id = room["id"].as_i64().expect("room id");

    let response = send_request(
        app.clone(),
        create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/tokens"),
            Some(Body::from(json!({}).to_string())),
        ),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let token: Value = assert_json(response).await?;
    Ok((
        room_id,
        token["token"].as_str().expect("room token").to_owned(),
    ))
}

async fn insert_content(
    repository: &RoomContentRepository,
    room_id: i64,
    content_type: ContentType,
    sequence_number: i32,
    value: &str,
) -> Result<()> {
    let mut content = RoomContent::builder()
        .room_id(room_id)
        .content_type(content_type)
        .sequence_number(sequence_number)
        .now(Utc::now().naive_utc())
        .build();
    match content_type {
        ContentType::Text => content.set_text(value.to_owned()),
        ContentType::Url => content.set_url(value.to_owned(), Some("text/html".to_owned())),
        ContentType::Image | ContentType::File => unreachable!("not used by this fixture"),
    }
    repository.create(&content).await?;
    Ok(())
}

#[tokio::test]
async fn message_pages_use_stable_keyset_order_and_exclude_non_text_content() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let room_name = "message_page_order";
    let (room_id, token) = create_room_and_token(&app, room_name).await?;
    let repository = RoomContentRepository::new(pool);

    for (sequence_number, text) in [
        (0, "message-0a"),
        (0, "message-0b"),
        (1, "message-1"),
        (2, "message-2"),
        (3, "message-3"),
        (4, "message-4"),
        (5, "message-5"),
    ] {
        insert_content(
            &repository,
            room_id,
            ContentType::Text,
            sequence_number,
            text,
        )
        .await?;
    }
    insert_content(
        &repository,
        room_id,
        ContentType::Url,
        99,
        "https://example.com",
    )
    .await?;

    let first_response = send_request(
        app.clone(),
        create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/messages?token={token}&limit=3"),
            None,
        ),
    )
    .await?;
    assert_eq!(first_response.status(), StatusCode::OK);
    let first: MessagePage = assert_json(first_response).await?;
    let first_texts: Vec<_> = first
        .items
        .iter()
        .map(|item| item.text.as_deref().expect("text message"))
        .collect();
    assert_eq!(first_texts, ["message-3", "message-4", "message-5"]);
    assert!(first.has_more);
    assert_eq!(first.next_sequence_number, 6);

    let second_response = send_request(
        app.clone(),
        create_request(
            Method::GET,
            &format!(
                "/api/v1/rooms/{room_name}/messages?token={token}&limit=3&cursor={}",
                first.next_cursor.as_deref().expect("next cursor")
            ),
            None,
        ),
    )
    .await?;
    assert_eq!(second_response.status(), StatusCode::OK);
    let second: MessagePage = assert_json(second_response).await?;
    let second_texts: Vec<_> = second
        .items
        .iter()
        .map(|item| item.text.as_deref().expect("text message"))
        .collect();
    assert_eq!(second_texts, ["message-0b", "message-1", "message-2"]);
    assert!(second.has_more);

    let third_response = send_request(
        app,
        create_request(
            Method::GET,
            &format!(
                "/api/v1/rooms/{room_name}/messages?token={token}&limit=3&cursor={}",
                second.next_cursor.as_deref().expect("next cursor")
            ),
            None,
        ),
    )
    .await?;
    assert_eq!(third_response.status(), StatusCode::OK);
    let third: MessagePage = assert_json(third_response).await?;
    assert_eq!(third.items.len(), 1);
    assert_eq!(third.items[0].text.as_deref(), Some("message-0a"));
    assert!(!third.has_more);
    assert!(third.next_cursor.is_none());

    let mut ids = first
        .items
        .iter()
        .chain(second.items.iter())
        .chain(third.items.iter())
        .map(|item| item.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 7);
    Ok(())
}

#[tokio::test]
async fn message_page_rejects_invalid_limit_and_cursor() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "message_page_validation";
    let (_room_id, token) = create_room_and_token(&app, room_name).await?;

    for query in ["limit=0", "limit=101", "cursor=invalid", "cursor=1:0"] {
        let response = send_request(
            app.clone(),
            create_request(
                Method::GET,
                &format!("/api/v1/rooms/{room_name}/messages?token={token}&{query}"),
                None,
            ),
        )
        .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "query={query}");
    }

    Ok(())
}
