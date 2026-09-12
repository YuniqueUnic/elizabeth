//! 内容寻址去重与秒传（issue #200）：
//! per-room 去重、秒传命中/谎报回退、引用计数归零物理清理、跨房不共享。

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
    Ok(body_json(response).await?["token"]
        .as_str()
        .expect("admin token")
        .to_string())
}

fn put_request(
    room: &str,
    filename: &str,
    token: &str,
    body: &'static [u8],
) -> Result<Request<Body>> {
    Ok(Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/rooms/{room}/files/{filename}"))
        .header("content-length", body.len().to_string())
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(body))?)
}

async fn put_upload(
    app: &axum::Router,
    room: &str,
    filename: &str,
    token: &str,
    body: &'static [u8],
) -> Result<Value> {
    let response = app
        .clone()
        .oneshot(put_request(room, filename, token, body)?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[tokio::test]
async fn test_same_room_duplicate_upload_dedupes_to_single_object() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = create_room(&app, "dedup-room").await?;
    let payload: &'static [u8] = b"duplicate content payload";

    let first = put_upload(&app, "dedup-room", "one.bin", &token, payload).await?;
    let second = put_upload(&app, "dedup-room", "two.bin", &token, payload).await?;

    // 两条内容记录，同一 locator 与 hash
    let rows: Vec<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT path, hash FROM room_contents WHERE room_id = (SELECT id FROM rooms WHERE name = 'dedup-room') ORDER BY id")
            .fetch_all(pool.as_ref())
            .await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], rows[1], "同房间重复内容必须共享同一物理对象");
    assert_eq!(rows[0].1.as_deref(), Some(sha256_hex(payload).as_str()));

    // blob 引用计数为 2
    let ref_count: i64 = sqlx::query_scalar(
        "SELECT ref_count FROM room_content_blobs WHERE room_id = (SELECT id FROM rooms WHERE name = 'dedup-room')",
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(ref_count, 2);

    // 物理对象只有一份
    let blob_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM room_content_blobs WHERE room_id = (SELECT id FROM rooms WHERE name = 'dedup-room')",
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(blob_count, 1);
    Ok(())
}

#[tokio::test]
async fn test_instant_upload_with_correct_hash_skips_transfer() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = create_room(&app, "instant-room").await?;
    let payload: &'static [u8] = b"instant upload payload";
    put_upload(&app, "instant-room", "original.txt", &token, payload).await?;

    // prepare 携带正确哈希：无需预留，直接建账
    let prepare = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/instant-room/contents/prepare?token={token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"files": [{"name": "copy.txt", "size": payload.len(), "mime": "text/plain", "file_hash": sha256_hex(payload)}]})
                .to_string(),
        ))?;
    let response = app.clone().oneshot(prepare).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let prepared = body_json(response).await?;
    assert_eq!(
        prepared["reservation_id"],
        Value::Null,
        "全部秒传命中时不应有预留"
    );
    let instant = prepared["instant_uploads"]
        .as_array()
        .expect("instant uploads");
    assert_eq!(instant.len(), 1);
    assert_eq!(instant[0]["file_name"], json!("copy.txt"));
    let download_url = instant[0]["download_url"].as_str().expect("download url");

    // 秒传内容立即可下载且字节一致
    let content_id = instant[0]["id"].as_i64().expect("content id");
    let download = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/contents/{content_id}?token={token}"))
        .body(Body::empty())?;
    let response = app.clone().oneshot(download).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    assert_eq!(&bytes[..], payload);
    assert!(download_url.ends_with(&format!("/contents/{content_id}")));

    // 引用计数 +1、房间配额按内容行累计
    let ref_count: i64 = sqlx::query_scalar(
        "SELECT ref_count FROM room_content_blobs WHERE room_id = (SELECT id FROM rooms WHERE name = 'instant-room')",
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(ref_count, 2);
    let current_size: i64 =
        sqlx::query_scalar("SELECT current_size FROM rooms WHERE name = 'instant-room'")
            .fetch_one(pool.as_ref())
            .await?;
    assert_eq!(current_size, 2 * payload.len() as i64);
    Ok(())
}

#[tokio::test]
async fn test_instant_upload_with_wrong_hash_falls_back_to_normal_upload() -> Result<()> {
    let (app, _pool) = create_test_app().await?;
    let token = create_room(&app, "fallback-room").await?;
    let payload: &'static [u8] = b"honest payload";
    put_upload(&app, "fallback-room", "real.bin", &token, payload).await?;

    // 谎报哈希：命中失败，回退正常预留上传
    let fake_hash = "0".repeat(64);
    let prepare = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/fallback-room/contents/prepare?token={token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"files": [{"name": "fake.bin", "size": payload.len(), "mime": "application/octet-stream", "file_hash": fake_hash}]})
                .to_string(),
        ))?;
    let response = app.clone().oneshot(prepare).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let prepared = body_json(response).await?;
    assert!(
        prepared["reservation_id"].as_i64().is_some(),
        "未命中必须回退到预留上传"
    );
    assert!(prepared.get("instant_uploads").is_none());

    // 大小不符同样不算命中（哈希正确但大小对不上）
    let prepare = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rooms/fallback-room/contents/prepare?token={token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"files": [{"name": "mismatch.bin", "size": payload.len() + 1, "mime": "application/octet-stream", "file_hash": sha256_hex(payload)}]})
                .to_string(),
        ))?;
    let response = app.clone().oneshot(prepare).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let prepared = body_json(response).await?;
    assert!(prepared["reservation_id"].as_i64().is_some());

    // 回退后的正常上传照常工作
    let upload = put_upload(&app, "fallback-room", "again.bin", &token, payload).await?;
    assert_eq!(upload["uploaded"][0]["file_name"], json!("again.bin"));
    Ok(())
}

#[tokio::test]
async fn test_delete_releases_references_and_cleans_object_at_zero() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token = create_room(&app, "gc-room").await?;
    let payload: &'static [u8] = b"to be garbage collected";

    let first = put_upload(&app, "gc-room", "a.bin", &token, payload).await?;
    let second = put_upload(&app, "gc-room", "b.bin", &token, payload).await?;
    let first_id = first["uploaded"][0]["id"].as_i64().unwrap();
    let second_id = second["uploaded"][0]["id"].as_i64().unwrap();
    let object_path: String = sqlx::query_scalar("SELECT path FROM room_contents WHERE id = $1")
        .bind(first_id)
        .fetch_one(pool.as_ref())
        .await?;

    // 删除第一份：仍有一处引用，物理对象保留
    let delete = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/rooms/gc-room/contents?token={token}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({"ids": [first_id]}).to_string()))?;
    let response = app.clone().oneshot(delete).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(tokio::fs::try_exists(&object_path).await.unwrap());
    let ref_count: i64 = sqlx::query_scalar("SELECT ref_count FROM room_content_blobs")
        .fetch_one(pool.as_ref())
        .await?;
    assert_eq!(ref_count, 1);

    // 删除第二份：引用归零，物理对象被清理
    let delete = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/rooms/gc-room/contents?token={token}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({"ids": [second_id]}).to_string()))?;
    let response = app.clone().oneshot(delete).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!tokio::fs::try_exists(&object_path).await.unwrap());
    let blob_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM room_content_blobs")
        .fetch_one(pool.as_ref())
        .await?;
    assert_eq!(blob_count, 0);
    Ok(())
}

#[tokio::test]
async fn test_cross_room_dedup_is_disabled_by_default() -> Result<()> {
    let (app, pool) = create_test_app().await?;
    let token_a = create_room(&app, "room-a").await?;
    let token_b = create_room(&app, "room-b").await?;
    let payload: &'static [u8] = b"shared across rooms payload";

    let in_a = put_upload(&app, "room-a", "shared.bin", &token_a, payload).await?;

    // room-a 已有同内容，room-b 的秒传探测必须未命中（跨房间不共享）
    let probe = |token: &str| {
        Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/v1/rooms/room-{}/contents/prepare?token={token}",
                if token == token_a { "a" } else { "b" }
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"files": [{"name": "probe.bin", "size": payload.len(), "mime": "application/octet-stream", "file_hash": sha256_hex(payload)}]})
                    .to_string(),
            ))
            .unwrap()
    };
    let response = app.clone().oneshot(probe(&token_b)).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let prepared = body_json(response).await?;
    assert!(
        prepared.get("instant_uploads").is_none(),
        "跨房间不允许秒传命中：{prepared}"
    );

    // room-b 上传后，同房间第二次秒传命中
    let in_b = put_upload(&app, "room-b", "shared.bin", &token_b, payload).await?;
    let response = app.clone().oneshot(probe(&token_b)).await?;
    let prepared = body_json(response).await?;
    assert!(
        prepared.get("instant_uploads").is_some(),
        "同房间第二次上传应秒传命中"
    );

    let locators: Vec<String> = sqlx::query_scalar(
        "SELECT path FROM room_contents WHERE room_id IN (SELECT id FROM rooms WHERE name IN ('room-a', 'room-b')) ORDER BY id",
    )
    .fetch_all(pool.as_ref())
    .await?;
    // room-a shared.bin + room-b shared.bin + room-b 秒传命中的 probe.bin
    assert_eq!(locators.len(), 3);
    assert_ne!(locators[0], locators[1], "跨房间默认不共享物理对象");

    let _ = (&in_a, &in_b);
    Ok(())
}

#[tokio::test]
async fn test_content_update_persists_hash_and_hidden_flag() -> Result<()> {
    // 回归：UPDATE 语句曾因占位符与绑定错位而静默失效（隐藏不落库）。
    let (app, pool) = create_test_app().await?;
    let token = create_room(&app, "update-room").await?;
    let payload: &'static [u8] = b"update persistence probe";
    let uploaded = put_upload(&app, "update-room", "probe.txt", &token, payload).await?;
    let content_id = uploaded["uploaded"][0]["id"].as_i64().unwrap();

    let set_hidden = Request::builder()
        .method(Method::PUT)
        .uri(format!(
            "/api/v1/rooms/update-room/contents/{content_id}/visibility?token={token}"
        ))
        .header("content-type", "application/json")
        .body(Body::from(json!({"hidden": true}).to_string()))?;
    let response = app.clone().oneshot(set_hidden).await?;
    assert_eq!(response.status(), StatusCode::OK);

    let (hidden, hash): (i64, Option<String>) =
        sqlx::query_as("SELECT hidden, hash FROM room_contents WHERE id = $1")
            .bind(content_id)
            .fetch_one(pool.as_ref())
            .await?;
    assert_eq!(hidden, 1, "隐藏状态必须真实落库");
    assert_eq!(hash.as_deref(), Some(sha256_hex(payload).as_str()));
    Ok(())
}
