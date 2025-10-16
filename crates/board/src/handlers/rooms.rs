use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum_responses::http::HttpResponse;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::db::DbPool;
use crate::models::Room;
use crate::repository::{RoomRepository, SqliteRoomRepository};

type HandlerResult<T> = Result<Json<T>, HttpResponse>;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoomParams {
    password: Option<String>,
}

/// 创建房间
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("password" = Option<String>, Query, description = "房间密码")
    ),
    responses(
        (status = 200, description = "房间创建成功", body = Room),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn create(
    Path(name): Path<String>,
    Query(params): Query<CreateRoomParams>,
    State(pool): State<Arc<DbPool>>,
) -> HandlerResult<Room> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let repository = SqliteRoomRepository::new(pool.clone());

    if repository.exists(&name).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Database error: {}", e))
    })? {
        return Err(HttpResponse::BadRequest().message("Room already exists"));
    }

    let room = Room::new(name.clone(), params.password);
    let created_room = repository.create(&room).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to create room: {}", e))
    })?;

    Ok(Json(created_room))
}

/// 查找房间
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    responses(
        (status = 200, description = "房间信息", body = Room),
        (status = 403, description = "房间无法进入"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn find(
    Path(name): Path<String>,
    State(pool): State<Arc<DbPool>>,
) -> HandlerResult<Room> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let repository = SqliteRoomRepository::new(pool.clone());

    match repository.find_by_name(&name).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Database error: {}", e))
    })? {
        Some(room) => {
            if room.can_enter() {
                Ok(Json(room))
            } else {
                Err(HttpResponse::Forbidden().message("Room cannot be entered"))
            }
        }
        None => {
            // 如果房间不存在，创建一个新的
            let new_room = Room::new(name, None);
            let created_room = repository.create(&new_room).await.map_err(|e| {
                HttpResponse::InternalServerError().message(format!("Failed to create room: {}", e))
            })?;
            Ok(Json(created_room))
        }
    }
}

/// 删除房间
#[utoipa::path(
    delete,
    path = "/api/v1/rooms/{name}",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    responses(
        (status = 200, description = "房间删除成功"),
        (status = 404, description = "房间不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "rooms"
)]
pub async fn delete(
    Path(name): Path<String>,
    State(pool): State<Arc<DbPool>>,
) -> impl IntoResponse {
    if name.is_empty() {
        return HttpResponse::BadRequest()
            .message("Invalid room name")
            .into_response();
    }

    let repository = SqliteRoomRepository::new(pool.clone());

    match repository.delete(&name).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to delete room: {}", e))
    }) {
        Ok(true) => HttpResponse::Ok()
            .message("Room deleted successfully")
            .into_response(),
        Ok(false) => HttpResponse::NotFound()
            .message("Room not found")
            .into_response(),
        Err(e) => e.into_response(),
    }
}
