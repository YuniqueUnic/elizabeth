use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::common::{create_test_app, http::create_request};

async fn response_json(response: axum::response::Response) -> Result<Value> {
    Ok(serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX).await?,
    )?)
}

async fn redeem(
    app: &axum::Router,
    room_name: &str,
    code: &str,
) -> Result<axum::response::Response> {
    Ok(app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/identity-codes/redeem"),
            Some(Body::from(json!({ "code": code }).to_string())),
        ))
        .await?)
}

#[tokio::test]
async fn manager_can_replace_identity_codes_and_update_editor_ttl() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let room_name = "identity_code_management";
    let response = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}"),
            Some(Body::from(
                json!({ "password": "secret123", "admin_identity_code": "admin-code-001" })
                    .to_string(),
            )),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let created = response_json(response).await?;
    assert_eq!(created["identity_code"], "admin-code-001");
    let admin_token = created["token"].as_str().expect("admin token").to_string();

    let response = app
        .clone()
        .oneshot(create_request(
            Method::POST,
            &format!("/api/v1/rooms/{room_name}/identity-codes?token={admin_token}"),
            Some(Body::from(
                json!({ "code": "editor-code-001", "role": "editor", "expires_in_secs": 3600 })
                    .to_string(),
            )),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let editor = response_json(response).await?;
    let editor_id = editor["id"].as_i64().expect("editor identity code id");
    assert_eq!(editor["role"], "editor");
    assert_eq!(editor["is_current"], false);

    let response = app
        .clone()
        .oneshot(create_request(
            Method::GET,
            &format!("/api/v1/rooms/{room_name}/identity-codes?token={admin_token}"),
            None,
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let codes = response_json(response).await?;
    let admin_id = codes
        .as_array()
        .expect("identity code list")
        .iter()
        .find(|code| code["role"] == "admin" && code["is_current"] == true)
        .and_then(|code| code["id"].as_i64())
        .expect("current admin identity code id");

    let response = app
        .clone()
        .oneshot(create_request(
            Method::PATCH,
            &format!("/api/v1/rooms/{room_name}/identity-codes/{editor_id}?token={admin_token}"),
            Some(Body::from(
                json!({ "code": "editor-code-002", "expires_in_secs": 120 }).to_string(),
            )),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_editor = response_json(response).await?;
    assert_eq!(updated_editor["code"], "editor-code-002");
    assert_eq!(updated_editor["role"], "editor");

    let old_editor = redeem(&app, room_name, "editor-code-001").await?;
    assert_eq!(old_editor.status(), StatusCode::UNAUTHORIZED);
    let new_editor = redeem(&app, room_name, "editor-code-002").await?;
    assert_eq!(new_editor.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(create_request(
            Method::PATCH,
            &format!("/api/v1/rooms/{room_name}/identity-codes/{admin_id}?token={admin_token}"),
            Some(Body::from(json!({ "code": "vip-admin-001" }).to_string())),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_admin = response_json(response).await?;
    assert_eq!(updated_admin["code"], "vip-admin-001");
    assert_eq!(updated_admin["is_current"], true);

    let old_admin = redeem(&app, room_name, "admin-code-001").await?;
    assert_eq!(old_admin.status(), StatusCode::UNAUTHORIZED);
    let new_admin = redeem(&app, room_name, "vip-admin-001").await?;
    assert_eq!(new_admin.status(), StatusCode::OK);
    Ok(())
}
