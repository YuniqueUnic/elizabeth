use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use url::Url;

use crate::dto::content::{CreateUrlContentRequest, CreateUrlContentResponse, RoomContentView};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::content::{ContentType, RoomContent};
use crate::repository::{
    IRoomContentRepository, IRoomRepository, RoomContentRepository, RoomRepository,
};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::{ContentPermission, HandlerResult, ensure_permission, room_id_or_error};

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
    let content = build_url_content(room_id, display_name, url, payload.description);
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

    broadcast_content_created(app_state, name, saved_content.clone());

    Ok(Json(CreateUrlContentResponse {
        created: RoomContentView::from(saved_content),
    }))
}

fn build_url_content(
    room_id: i64,
    display_name: &str,
    url: &str,
    description: Option<String>,
) -> RoomContent {
    let now = chrono::Utc::now().naive_utc();
    let mut content = RoomContent::builder()
        .room_id(room_id)
        .content_type(ContentType::Url)
        .sequence_number(0)
        .now(now)
        .build();
    content.file_name = Some(display_name.to_string());
    content.text = description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());
    content.set_url(url.to_string(), Some("text/html".to_string()));
    content
}

fn broadcast_content_created(app_state: Arc<AppState>, room_name: String, content: RoomContent) {
    let broadcaster = app_state.broadcaster.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_content_created(&room_name, &content)
            .await
        {
            log::warn!("Failed to broadcast URL content created event: {}", e);
        }
    });
}
