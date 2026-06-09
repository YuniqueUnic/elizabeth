use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::response::Response;
use chrono::NaiveDateTime;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use tokio_util::io::ReaderStream;
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::constants::{
    storage::DEFAULT_STORAGE_ROOT, upload::DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
};
use crate::dto::content::{
    CreateMessageRequest, CreateMessageResponse, CreateUrlContentRequest, CreateUrlContentResponse,
    DeleteContentRequest, DeleteContentResponse, RoomContentView, UpdateContentRequest,
    UpdateContentResponse, UploadContentResponse, UploadPreparationRequest,
    UploadPreparationResponse,
};
use crate::errors::{AppError, AppResult};
use crate::models::{
    UploadFileDescriptor,
    content::{ContentType, RoomContent},
};
use crate::repository::{
    IRoomContentRepository, IRoomRepository, IRoomUploadReservationRepository,
    RoomContentRepository, RoomRepository, RoomUploadReservationRepository,
};
use crate::services::RoomTokenClaims;
use crate::state::AppState;
use crate::validation::{RoomNameValidator, TokenValidator};

use super::{AuthToken, verify_room_token, verify_room_token_by_id};

type HandlerResult<T> = Result<Json<T>, AppError>;

pub mod upload;

#[utoipa::path(
    delete,
    path = "/api/v1/rooms/{name}/contents",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = DeleteContentRequest,
    responses(
        (status = 200, description = "删除成功", body = DeleteContentResponse),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无删除权限"),
        (status = 404, description = "房间或文件不存在")
    ),
    tag = "content"
)]
pub async fn delete_contents(
    AxumPath(name): AxumPath<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<DeleteContentRequest>,
) -> HandlerResult<DeleteContentResponse> {
    // Validate room name using the new validation framework
    RoomNameValidator::validate_identifier(&name)?;

    if payload.ids.is_empty() {
        return Err(AppError::validation("No content id provided"));
    }

    let mut verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_delete(),
        ContentPermission::Delete,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let existing_contents = repository
        .list_by_room(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {e}")))?;

    let target_ids: HashSet<i64> = payload.ids.iter().copied().collect();
    let contents: Vec<RoomContent> = existing_contents
        .into_iter()
        .filter(|content| {
            content
                .id
                .map(|id| target_ids.contains(&id))
                .unwrap_or(false)
        })
        .collect();

    if contents.is_empty() {
        return Err(AppError::not_found("Contents not found"));
    }

    let mut freed_size = 0;
    for content in &contents {
        if let Some(path) = &content.path {
            fs::remove_file(path).await.ok();
        }
        freed_size += content.size.unwrap_or(0);
    }

    let ids: Vec<i64> = contents.iter().filter_map(|content| content.id).collect();

    repository
        .delete_by_ids(room_id, &ids)
        .await
        .map_err(|e| AppError::internal(format!("Delete failed: {e}")))?;

    if freed_size > 0 {
        verified.room.current_size = (verified.room.current_size - freed_size).max(0);
        let room_repo = RoomRepository::new(app_state.db_pool.clone());
        verified.room = room_repo
            .update(&verified.room)
            .await
            .map_err(|e| AppError::internal(format!("Update room failed: {e}")))?;
    }

    let ids_for_response = ids.clone();
    let deleted_contents = contents.clone();

    // 广播内容删除事件
    let broadcaster = app_state.broadcaster.clone();
    let room_name_clone = name.clone();
    tokio::spawn(async move {
        for content in &deleted_contents {
            if let Err(e) = broadcaster
                .broadcast_content_deleted(&room_name_clone, content)
                .await
            {
                log::warn!(
                    "Failed to broadcast content deleted event for {}: {}",
                    content.id.unwrap_or_default(),
                    e
                );
            }
        }
    });

    Ok(Json(DeleteContentResponse {
        deleted: ids_for_response,
        freed_size,
        current_size: verified.room.current_size,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}/contents/{content_id}",
    params(
        ("name" = String, Path, description = "房间名称"),
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
async fn serve_content_stream(content: RoomContent) -> Result<Response, AppError> {
    let path = content
        .path
        .ok_or_else(|| AppError::not_found("Content not stored on disk"))?;

    let file = fs::File::open(&path)
        .await
        .map_err(|_| AppError::not_found("File missing on disk"))?;

    // 使用数据库中的原始文件名，如果不存在则从路径提取
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
    // 验证令牌格式
    TokenValidator::validate_token_format(&token)?;

    // 查找资产
    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let content = repository
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {e}")))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    // 关键！不再依赖 path 中的 room_name，而是使用资产所属的真实 room_id 去校验 token
    let verified = verify_room_token_by_id(app_state.clone(), content.room_id, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    serve_content_stream(content).await
}

#[derive(Clone, Copy)]
enum ContentPermission {
    View,
    Edit,
    Delete,
}

fn ensure_permission(
    claims: &RoomTokenClaims,
    room_allows: bool,
    action: ContentPermission,
) -> Result<(), AppError> {
    if !room_allows {
        return Err(AppError::permission_denied("Permission denied by room"));
    }
    let permission = claims.as_permission();
    let token_allows = match action {
        ContentPermission::View => permission.can_view(),
        ContentPermission::Edit => permission.can_edit(),
        ContentPermission::Delete => permission.can_delete(),
    };
    if !token_allows {
        return Err(AppError::permission_denied("Permission denied by token"));
    }
    Ok(())
}

/// 确保房间存储目录存在，使用 room_id 作为目录名
async fn ensure_room_storage(base_dir: &Path, room_id: i64) -> Result<PathBuf, std::io::Error> {
    let dir = base_dir.join(room_id.to_string());
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}

fn room_id_or_error(claims: &RoomTokenClaims) -> Result<i64, AppError> {
    if claims.room_id <= 0 {
        return Err(AppError::internal("Room id missing in claims"));
    }
    Ok(claims.room_id)
}

#[utoipa::path(
    put,
    path = "/api/v1/rooms/{name}/contents/{content_id}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("content_id" = i64, Path, description = "内容 ID"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = UpdateContentRequest,
    responses(
        (status = 200, description = "更新成功", body = UpdateContentResponse),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无编辑权限"),
        (status = 404, description = "房间或内容不存在"),
        (status = 409, description = "版本冲突")
    ),
    tag = "content"
)]
pub async fn update_content(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateContentRequest>,
) -> HandlerResult<UpdateContentResponse> {
    // Validate room name using new validation framework
    RoomNameValidator::validate_identifier(&name)?;

    // Validate content_id
    if content_id <= 0 {
        return Err(AppError::validation("Invalid content ID"));
    }

    // Validate payload - at least one field should be provided
    if payload.text.is_none() && payload.url.is_none() {
        return Err(AppError::validation(
            "At least one of text or url must be provided",
        ));
    }

    let mut verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let repository = RoomContentRepository::new(app_state.db_pool.clone());

    // Get existing content
    let existing_content = repository
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {e}")))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    // Verify content belongs to the room
    if existing_content.room_id != room_id {
        return Err(AppError::permission_denied("Content not in this room"));
    }

    // Version control check - compare timestamps
    let existing_timestamp = existing_content.timestamp();

    // Check for concurrent modification
    /*
    let current_timestamp = chrono::Utc::now().naive_utc().and_utc().timestamp();
    if payload.text.is_some() {
        // For text content updates, we'll implement optimistic concurrency control
        // In a real implementation, you might want to add version field to the request
        // For now, we'll allow updates but warn about potential conflicts
        if existing_timestamp > current_timestamp - 300 {
            // 5 minute window
            return Err(AppError::conflict(
                "Content was modified recently, please refresh and try again",
            ));
        }
    }
    */

    // Create updated content
    let now = chrono::Utc::now().naive_utc();
    let mut updated_content = existing_content.clone();
    updated_content.updated_at = now;

    // Update fields based on payload
    if let Some(text) = payload.text {
        updated_content.set_text(text);
    }

    if let Some(url) = payload.url {
        updated_content.set_url(url, payload.mime_type.clone());
    }

    // Save updated content
    let saved_content = repository
        .update(&updated_content)
        .await
        .map_err(|e| AppError::internal(format!("Update failed: {e}")))?;

    verified.room = room_repo_update_if_content_size_changed(
        &verified.room,
        &saved_content,
        &app_state.db_pool,
    )
    .await?;

    let saved_content_clone = saved_content.clone();

    // 广播内容更新事件
    let broadcaster = app_state.broadcaster.clone();
    let room_name_clone = name.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_content_updated(&room_name_clone, &saved_content_clone)
            .await
        {
            log::warn!("Failed to broadcast content updated event: {}", e);
        }
    });

    Ok(Json(UpdateContentResponse {
        updated: RoomContentView::from(saved_content),
    }))
}

// ============================================================================
// URL Content Creation API
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/contents/url",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = CreateUrlContentRequest,
    responses(
        (status = 200, description = "链接创建成功", body = CreateUrlContentResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无编辑权限或容量不足"),
        (status = 404, description = "房间不存在"),
        (status = 413, description = "超出房间容量限制")
    ),
    tag = "content"
)]
pub async fn create_url_content(
    AxumPath(name): AxumPath<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateUrlContentRequest>,
) -> HandlerResult<CreateUrlContentResponse> {
    RoomNameValidator::validate_identifier(&name)?;

    let url = payload.url.trim();
    if url.is_empty() {
        return Err(AppError::validation("URL cannot be empty"));
    }
    if Url::parse(url).is_err() {
        return Err(AppError::validation("URL must be absolute and valid"));
    }

    let display_name = payload.name.trim();
    if display_name.is_empty() {
        return Err(AppError::validation("URL name cannot be empty"));
    }

    let mut verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let now = chrono::Utc::now().naive_utc();
    let mut content = RoomContent::builder()
        .room_id(room_id)
        .content_type(ContentType::Url)
        .sequence_number(0)
        .now(now)
        .build();
    content.file_name = Some(display_name.to_string());
    content.text = payload
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());
    content.set_url(url.to_string(), Some("text/html".to_string()));

    let content_size = content.size.unwrap_or(0);
    if !verified.room.can_add_content(content_size) {
        return Err(AppError::payload_too_large("Room size limit exceeded"));
    }

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let saved_content = repository
        .create(&content)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create URL content: {e}")))?;

    verified.room.current_size += content_size;
    let room_repository = RoomRepository::new(app_state.db_pool.clone());
    verified.room = room_repository
        .update(&verified.room)
        .await
        .map_err(|e| AppError::internal(format!("Update room failed: {e}")))?;

    let saved_content_clone = saved_content.clone();
    let broadcaster = app_state.broadcaster.clone();
    let room_name_clone = name.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_content_created(&room_name_clone, &saved_content_clone)
            .await
        {
            log::warn!("Failed to broadcast URL content created event: {}", e);
        }
    });

    Ok(Json(CreateUrlContentResponse {
        created: RoomContentView::from(saved_content),
    }))
}

// Helper function to update room size when content size changes
async fn room_repo_update_if_content_size_changed(
    room: &crate::models::room::Room,
    content: &RoomContent,
    db_pool: &Arc<crate::db::DbPool>,
) -> Result<crate::models::room::Room, AppError> {
    use crate::repository::{IRoomRepository, RoomRepository};

    // Check if size actually changed
    let size_changed = match content.size {
        Some(content_size) => content_size != room.current_size,
        None => false,
    };

    if size_changed {
        let room_repo = RoomRepository::new(db_pool.clone());
        room_repo
            .update(room)
            .await
            .map_err(|e| AppError::internal(format!("Update room failed: {e}")))
    } else {
        Ok(room.clone())
    }
}

// ============================================================================
// Message Creation API
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/messages",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = CreateMessageRequest,
    responses(
        (status = 200, description = "消息创建成功", body = CreateMessageResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无发送消息权限"),
        (status = 404, description = "房间不存在")
    ),
    tag = "content"
)]
pub async fn create_message(
    AxumPath(name): AxumPath<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateMessageRequest>,
) -> HandlerResult<CreateMessageResponse> {
    // Validate room name using the new validation framework
    RoomNameValidator::validate_identifier(&name)?;

    // Validate message text
    let text = payload.text.trim();
    if text.is_empty() {
        return Err(AppError::validation("Message text cannot be empty"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let now = chrono::Utc::now().naive_utc();

    let seq = payload.sequence_number.unwrap_or(0);

    // Create message content using ContentType.Text
    let mut content = RoomContent::builder()
        .room_id(room_id)
        .content_type(ContentType::Text)
        .sequence_number(seq)
        .now(now)
        .build();

    content.set_text(text.to_string());

    // Save to database
    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let saved_content = repository
        .create(&content)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create message: {e}")))?;

    let saved_content_clone = saved_content.clone();

    // 广播内容创建事件
    let broadcaster = app_state.broadcaster.clone();
    let room_name_clone = name.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_content_created(&room_name_clone, &saved_content_clone)
            .await
        {
            log::warn!("Failed to broadcast message created event: {}", e);
        }
    });

    Ok(Json(CreateMessageResponse {
        message: RoomContentView::from(saved_content),
    }))
}
