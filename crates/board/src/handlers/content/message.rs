use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};

use crate::dto::content::{CreateMessageRequest, CreateMessageResponse, RoomContentView};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::content::{ContentType, RoomContent};
use crate::repository::{IRoomContentRepository, RoomContentRepository};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::{ContentPermission, HandlerResult, ensure_permission, room_id_or_error};

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
    RoomNameValidator::validate_identifier(&name)?;

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
    let content = build_message_content(room_id, text, payload.sequence_number.unwrap_or(0));

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let saved_content = repository
        .create(&content)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create message: {e}")))?;

    broadcast_content_created(app_state, name, saved_content.clone());

    Ok(Json(CreateMessageResponse {
        message: RoomContentView::from(saved_content),
    }))
}

fn build_message_content(room_id: i64, text: &str, sequence_number: i32) -> RoomContent {
    let now = chrono::Utc::now().naive_utc();
    let mut content = RoomContent::builder()
        .room_id(room_id)
        .content_type(ContentType::Text)
        .sequence_number(sequence_number)
        .now(now)
        .build();
    content.set_text(text.to_string());
    content
}

fn broadcast_content_created(app_state: Arc<AppState>, room_name: String, content: RoomContent) {
    let broadcaster = app_state.broadcaster.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_content_created(&room_name, &content)
            .await
        {
            log::warn!("Failed to broadcast message created event: {}", e);
        }
    });
}
