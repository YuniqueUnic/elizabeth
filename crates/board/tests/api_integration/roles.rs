#![allow(dead_code)]

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{create_test_app, http::create_request as create_http_request};

async fn issue_manager_token(app: &axum::Router, room_name: &str) -> Result<String> {
    let response = app
        .clone()
        .oneshot(create_http_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/tokens"),
            Some(Body::from(
                json!({"password": "secret123", "role": "admin"}).to_string(),
            )),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice::<serde_json::Value>(&body)?["token"]
        .as_str()
        .expect("manager token")
        .to_string())
}

async fn create_room(app: &axum::Router, room_name: &str) -> Result<String> {
    let response = app
        .clone()
        .oneshot(create_http_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}"),
            Some(Body::from(json!({"password": "secret123"}).to_string())),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice::<serde_json::Value>(&body)?["token"]
        .as_str()
        .expect("admin token")
        .to_string())
}

#[tokio::test]
async fn role_crud_and_capabilities_are_room_scoped() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "roles_test_room";
    let manager_token = create_room(&app, room_name).await?;

    let create = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{room_name}/roles?token={manager_token}"),
        Some(Body::from(
            json!({
                "role_key": "moderator",
                "display_name": "Moderator",
                "capabilities": [
                    {"capability": "msg.read", "scope": "any"},
                    {"capability": "msg.delete", "scope": "own"}
                ]
            })
            .to_string(),
        )),
    );
    let response = app.clone().oneshot(create).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let role: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(role["role_key"], "moderator");
    assert_eq!(role["capabilities"][0]["capability"], "msg.read");
    assert_eq!(role["capabilities"][1]["capability"], "msg.delete");
    assert_eq!(role["capabilities"][1]["scope"], "own");
    assert_eq!(role["is_system"], false);

    let mut list = create_http_request(
        Method::GET,
        &format!("/api/v1/rooms/{room_name}/roles"),
        None,
    );
    list.headers_mut().insert(
        "x-api-key",
        manager_token.parse().expect("valid identity code header"),
    );
    let response = app.clone().oneshot(list).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let roles: Vec<serde_json::Value> = serde_json::from_slice(&body)?;
    assert!(roles.iter().any(|role| role["role_key"] == "moderator"));

    let update = create_http_request(
        Method::PUT,
        &format!("/api/v1/rooms/{room_name}/roles/moderator?token={manager_token}"),
        Some(Body::from(
            json!({
                "display_name": "Moderator",
                "capabilities": [
                    {"capability": "msg.read", "scope": "any"},
                    {"capability": "msg.copy", "scope": "any"},
                    {"capability": "msg.delete", "scope": "any"}
                ]
            })
            .to_string(),
        )),
    );
    let response = app.clone().oneshot(update).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let updated: serde_json::Value = serde_json::from_slice(&body)?;
    assert!(
        updated["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|grant| grant["capability"] == "msg.delete" && grant["scope"] == "any")
    );

    let delete = create_http_request(
        Method::DELETE,
        &format!("/api/v1/rooms/{room_name}/roles/moderator?token={manager_token}"),
        None,
    );
    let response = app.clone().oneshot(delete).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn reader_cannot_manage_roles() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "roles_reader_room";
    let _admin_token = create_room(&app, room_name).await?;
    let response = app
        .clone()
        .oneshot(create_http_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/tokens"),
            Some(Body::from(
                json!({"password": "secret123", "role": "reader"}).to_string(),
            )),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let reader_token = serde_json::from_slice::<serde_json::Value>(&body)?["token"]
        .as_str()
        .expect("reader token")
        .to_string();

    let response = app
        .oneshot(create_http_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/roles?token={reader_token}"),
            None,
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    Ok(())
}
