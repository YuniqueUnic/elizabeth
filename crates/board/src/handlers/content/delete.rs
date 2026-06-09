use std::collections::HashSet;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};

use crate::dto::content::{DeleteContentRequest, DeleteContentResponse};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::content::RoomContent;
use crate::repository::{
    IRoomContentRepository, IRoomRepository, RoomContentRepository, RoomRepository,
};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::{ContentPermission, HandlerResult, ensure_permission, room_id_or_error};

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

    let contents = collect_target_contents(existing_contents, &payload.ids);
    if contents.is_empty() {
        return Err(AppError::not_found("Contents not found"));
    }

    let freed_size = remove_content_files(&contents).await;
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

    broadcast_content_deleted(app_state.clone(), name, contents);

    Ok(Json(DeleteContentResponse {
        deleted: ids,
        freed_size,
        current_size: verified.room.current_size,
    }))
}

fn collect_target_contents(contents: Vec<RoomContent>, target_ids: &[i64]) -> Vec<RoomContent> {
    let target_ids: HashSet<i64> = target_ids.iter().copied().collect();
    contents
        .into_iter()
        .filter(|content| {
            content
                .id
                .map(|id| target_ids.contains(&id))
                .unwrap_or(false)
        })
        .collect()
}

async fn remove_content_files(contents: &[RoomContent]) -> i64 {
    let mut freed_size = 0;
    for content in contents {
        if let Some(path) = &content.path {
            tokio::fs::remove_file(path).await.ok();
        }
        freed_size += content.size.unwrap_or(0);
    }
    freed_size
}

fn broadcast_content_deleted(
    app_state: Arc<AppState>,
    room_name: String,
    contents: Vec<RoomContent>,
) {
    let broadcaster = app_state.broadcaster.clone();
    tokio::spawn(async move {
        for content in &contents {
            if let Err(e) = broadcaster
                .broadcast_content_deleted(&room_name, content)
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
}
