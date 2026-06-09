use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::response::Response;
use tokio::fs;
use tokio_util::io::ReaderStream;

use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token_by_id};
use crate::models::content::RoomContent;
use crate::repository::{IRoomContentRepository, RoomContentRepository};
use crate::state::AppState;
use crate::validation::TokenValidator;

use super::{ContentPermission, ensure_permission};

#[utoipa::path(
    get,
    path = "/api/v1/contents/{content_id}",
    params(
        ("content_id" = i64, Path, description = "内容 id"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    responses(
        (status = 200, description = "文件内容"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无访问权限"),
        (status = 404, description = "文件不存在")
    ),
    tag = "content"
)]
pub async fn download_content_global(
    AxumPath(content_id): AxumPath<i64>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> Result<Response, AppError> {
    TokenValidator::validate_token_format(&token)?;

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let content = repository
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {e}")))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    let verified = verify_room_token_by_id(app_state.clone(), content.room_id, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    serve_content_stream(content).await
}

async fn serve_content_stream(content: RoomContent) -> Result<Response, AppError> {
    let path = content
        .path
        .ok_or_else(|| AppError::not_found("Content not stored on disk"))?;

    let file = fs::File::open(&path)
        .await
        .map_err(|_| AppError::not_found("File missing on disk"))?;

    let file_name = content.file_name.clone().unwrap_or_else(|| {
        Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "download.bin".to_string())
    });

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let mut response = Response::new(body);
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
        .map_err(|_| AppError::internal("Failed to build response headers"))?;
    response
        .headers_mut()
        .insert(CONTENT_DISPOSITION, disposition);

    if let Some(size) = content.size
        && let Ok(value) = HeaderValue::from_str(&size.to_string())
    {
        response.headers_mut().insert(CONTENT_LENGTH, value);
    }

    if let Some(mime) = content.mime_type
        && let Ok(value) = HeaderValue::from_str(&mime)
    {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }

    Ok(response)
}
