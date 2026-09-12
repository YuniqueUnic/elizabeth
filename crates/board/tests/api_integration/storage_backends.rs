//! 存储后端无关性（issue #197）：把 Opendal 后端（locator = 对象 key）注入
//! AppState，验证上传 / 下载 / PUT / 删除等 HTTP 流程在非文件系统后端下行为一致。

use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use board::storage::OpendalBackend;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;
use uuid::Uuid;

use crate::common::http::{assert_status, create_request as create_http_request};

async fn body_json(response: axum::response::Response) -> Result<Value> {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[tokio::test]
async fn test_http_flows_work_against_opendal_backend() -> Result<()> {
    // 内存库 + Opendal 后端（locator = 对象 key）。
    // 独立临时目录承载 opendal Fs operator；TempDir 保持存活到测试结束。
    let opendal_root = TempDir::new()?;
    let operator = opendal::Operator::new(
        opendal::services::Fs::default().root(opendal_root.path().to_str().unwrap()),
    )?;
    let storage: Arc<dyn board::storage::StorageBackend> =
        Arc::new(OpendalBackend::from_operator(operator));
    let (app, _pool) = crate::common::create_test_app_with_storage(Some(storage)).await?;

    // 建房
    let room_name = format!("opendal-room-{}", Uuid::new_v4().simple());
    let create = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{room_name}"),
        Some(Body::from(json!({}).to_string())),
    );
    let response = app.clone().oneshot(create).await?;
    assert_status(&response, StatusCode::OK);
    let token = body_json(response).await?["token"]
        .as_str()
        .expect("admin token")
        .to_string();

    // multipart 上传：locator 必须是对象 key（无前导 /）
    let boundary = "----opendal-backend-boundary";
    let mut multipart = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"notes.txt\"\r\nContent-Type: text/plain\r\n\r\n"
    )
    .into_bytes();
    multipart.extend_from_slice(b"opendal backend payload");
    multipart.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let prepare = create_http_request(
        Method::POST,
        &format!("/api/v1/rooms/{room_name}/contents/prepare?token={token}"),
        Some(Body::from(
            json!({"files": [{"name": "notes.txt", "size": 23, "mime": "text/plain"}]}).to_string(),
        )),
    );
    let response = app.clone().oneshot(prepare).await?;
    assert_status(&response, StatusCode::OK);
    let reservation_id = body_json(response).await?["reservation_id"]
        .as_i64()
        .expect("reservation id");

    let upload = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/rooms/{room_name}/contents?token={token}&reservation_id={reservation_id}"
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

    // 下载回读
    let download = create_http_request(
        Method::GET,
        &format!("/api/v1/contents/{content_id}?token={token}"),
        None,
    );
    let response = app.clone().oneshot(download).await?;
    assert_status(&response, StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    assert_eq!(&bytes[..], b"opendal backend payload");

    // PUT 单命令上传 + 回读
    let put = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/rooms/{room_name}/files/report.md"))
        .header("content-length", "5")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from("# ok\n".to_string()))?;
    let response = app.clone().oneshot(put).await?;
    assert_status(&response, StatusCode::OK);
    let put_json = body_json(response).await?;
    let report_id = put_json["uploaded"][0]["id"]
        .as_i64()
        .expect("put content id");
    assert!(
        put_json["uploaded"][0]["download_url"]
            .as_str()
            .unwrap()
            .ends_with(&format!("/contents/{report_id}"))
    );

    let download = create_http_request(
        Method::GET,
        &format!("/api/v1/contents/{report_id}?token={token}"),
        None,
    );
    let response = app.clone().oneshot(download).await?;
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    assert_eq!(&bytes[..], b"# ok\n");

    // 删除：Opendal delete 不应报错，且删除后下载 404
    let delete = create_http_request(
        Method::DELETE,
        &format!("/api/v1/rooms/{room_name}/contents?token={token}"),
        Some(Body::from(json!({"ids": [content_id]}).to_string())),
    );
    let response = app.clone().oneshot(delete).await?;
    assert_status(&response, StatusCode::OK);
    let download = create_http_request(
        Method::GET,
        &format!("/api/v1/contents/{content_id}?token={token}"),
        None,
    );
    let response = app.clone().oneshot(download).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
}
