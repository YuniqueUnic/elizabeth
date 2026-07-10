use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::dto::content::{
    CreateMessageRequest, CreateMessageResponse, MessagePage, RoomContentView,
};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::content::{ContentType, RoomContent};
use crate::repository::{IRoomContentRepository, MessagePageCursor, RoomContentRepository};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::{ContentPermission, HandlerResult, ensure_permission, room_id_or_error};

const DEFAULT_MESSAGE_PAGE_SIZE: u32 = 50;
const MAX_MESSAGE_PAGE_SIZE: u32 = 100;

#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct ListMessagesQuery {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}/messages",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token"),
        ListMessagesQuery
    ),
    responses(
        (status = 200, description = "按稳定游标分页的消息列表", body = MessagePage),
        (status = 400, description = "分页参数无效"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无查看消息权限"),
        (status = 404, description = "房间不存在")
    ),
    tag = "content"
)]
pub async fn list_messages(
    AxumPath(name): AxumPath<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<ListMessagesQuery>,
) -> HandlerResult<MessagePage> {
    RoomNameValidator::validate_identifier(&name)?;
    let limit = validate_page_limit(query.limit)?;
    let cursor = query.cursor.as_deref().map(parse_cursor).transpose()?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;
    let room_id = room_id_or_error(&verified.claims)?;

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let page = repository
        .list_messages_page(room_id, cursor, limit)
        .await
        .map_err(|error| AppError::internal(format!("Failed to list messages: {error}")))?;
    let next_cursor = if page.has_more {
        page.items.first().and_then(encode_cursor)
    } else {
        None
    };

    Ok(Json(MessagePage {
        items: page.items.into_iter().map(RoomContentView::from).collect(),
        next_cursor,
        has_more: page.has_more,
        next_sequence_number: page.next_sequence_number,
    }))
}

fn validate_page_limit(limit: Option<u32>) -> Result<u32, AppError> {
    let limit = limit.unwrap_or(DEFAULT_MESSAGE_PAGE_SIZE);
    if !(1..=MAX_MESSAGE_PAGE_SIZE).contains(&limit) {
        return Err(AppError::validation(format!(
            "limit must be between 1 and {MAX_MESSAGE_PAGE_SIZE}"
        )));
    }
    Ok(limit)
}

fn parse_cursor(cursor: &str) -> Result<MessagePageCursor, AppError> {
    let (sequence_number, id) = cursor
        .split_once(':')
        .ok_or_else(|| AppError::validation("Invalid message cursor"))?;
    let sequence_number = sequence_number
        .parse::<i32>()
        .map_err(|_| AppError::validation("Invalid message cursor"))?;
    let id = id
        .parse::<i64>()
        .map_err(|_| AppError::validation("Invalid message cursor"))?;
    if id <= 0 {
        return Err(AppError::validation("Invalid message cursor"));
    }
    Ok(MessagePageCursor {
        sequence_number,
        id,
    })
}

fn encode_cursor(content: &RoomContent) -> Option<String> {
    content
        .id
        .map(|id| format!("{}:{id}", content.sequence_number))
}

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
