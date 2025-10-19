use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum_responses::http::HttpResponse;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::db::DbPool;
use crate::models::Room;
use crate::repository::{IRoomContentRepository, SqliteRoomRepository};

// 这里是构建创建 room 中 content 的 handler 的地方
// 该过程需要有如下流程：
// 1. 该请求必须拿到 room 签发的一个凭证 (如 jwt), 然后使用该凭证去发起该 room 的 content 的一系列操作
//     1. 这个 jwt 只有在用户成功进入该 room 时才会签发，也就是如果该 room 有一个密码，那么只有用户成功进入该 room 才能获取到该凭证
//     2. 该 jwt 的有效期应该小于该 room 的有效期
//     3. 房间不存在时，jwt 也应该失效
// 2. 用户发起的 content 相关请求需要带上该凭证，然后使用该凭证去发起该 room 的内容相关的操作
// 3. 服务拿到用户的请求信息后，会先检查 jwt,
//     1. 服务拿到用户的请求信息后，会先检查 jwt，并解析出该 jwt 所对应的房间信息 (权限。当前所占 size) 以及 新文件 size 等等信息。
//     2. 如果这个 jwt 是有效的，有对应的 room, 那么就会开始检查 权限，检查允许的总 size <  当前 size + 新文件 size. 等等需要检查的内容。
//     3. 如果检查通过，那么就会开始处理该请求 (比如获取文件列表，上传文件等等)，并返回结果。
// 4. 用户得到服务端返回的结果
// content handler 应该有这些方法/接口
// 1. 获取某个房间下的文件列表
// 2. 上传文件 (s) 到某个房间
// 3. 删除某个房间下的某些文件
// 4. 下载某个房间下的某些文件
// room handler 也应该更新，增加一个签发凭证，验证凭证，删除凭证，获取凭证等等的常用方法/接口。
// 我们的这个程序是没有用户的概念的，只有 room 的概念，能够进入房间的都是能够对房间进行处理的 (但是房间的权限是由最开始创建的人配置的权限。后续的人是没有该权限的)
// 相关文件：
// /Users/unic/dev/projs/rs/elizabeth/crates/board/src/handlers
// /Users/unic/dev/projs/rs/elizabeth/crates/board/src/repository
// 此外关于 sqlx, 当前项目的常用构建等等操作，请你参考 /Users/unic/dev/projs/rs/elizabeth/justfile
// 请你最后实现和完成之后，参考 /Users/unic/dev/projs/rs/elizabeth/docs/great-blog/how-2-write-blog.md 写一篇博客，描述你的需求，分析，实现过程，痛点等等的技术性文章。
// 输出到 /Users/unic/dev/projs/rs/elizabeth/docs 中

// type HandlerResult<T> = Result<Json<T>, HttpResponse>;

// #[derive(Debug, Deserialize, ToSchema)]
// pub struct CreateTextParams {
//     jwt_token: Option<String>,
// }

// /// 创建房间
// #[utoipa::path(
//     post,
//     path = "/api/v1/content/text/{name}",
//     params(
//         ("name" = String, Path, description = "房间名称"),
//         ("jwt_token" = Option<String>, Query, description = "房间里的人的 jwt_token") // 应该使用 jwt 来替代，因为只有房间里的人 (拿到了 jwt) 才能更新内容
//     ),
//     responses(
//         (status = 200, description = "房间创建成功", body = Room),
//         (status = 400, description = "请求参数错误"),
//         (status = 500, description = "服务器内部错误")
//     ),
//     tag = "rooms"
// )]
// pub async fn create(
//     Path(name): Path<String>,
//     Query(params): Query<CreateTextParams>,
//     Body(text): Body<String>,
//     State(pool): State<Arc<DbPool>>,
// ) -> HandlerResult<Room> {
//     if name.is_empty() {
//         return Err(HttpResponse::BadRequest().message("Invalid room name"));
//     }

//     let repository = SqliteRoomRepository::new(pool.clone());

//     if repository.exists(&name).await.map_err(|e| {
//         HttpResponse::InternalServerError().message(format!("Database error: {}", e))
//     })? {
//         return Err(HttpResponse::BadRequest().message("Room already exists"));
//     }

//     let room = Room::new(name.clone(), params.password);
//     let created_room = repository.create(&room).await.map_err(|e| {
//         HttpResponse::InternalServerError().message(format!("Failed to create room: {}", e))
//     })?;

//     Ok(Json(created_room))
// }

// /// 查找房间
// #[utoipa::path(
//     get,
//     path = "/api/v1/rooms/{name}",
//     params(
//         ("name" = String, Path, description = "房间名称")
//     ),
//     responses(
//         (status = 200, description = "房间信息", body = Room),
//         (status = 403, description = "房间无法进入"),
//         (status = 500, description = "服务器内部错误")
//     ),
//     tag = "rooms"
// )]
// pub async fn find(
//     Path(name): Path<String>,
//     State(pool): State<Arc<DbPool>>,
// ) -> HandlerResult<Room> {
//     if name.is_empty() {
//         return Err(HttpResponse::BadRequest().message("Invalid room name"));
//     }

//     let repository = SqliteRoomRepository::new(pool.clone());

//     match repository.find_by_name(&name).await.map_err(|e| {
//         HttpResponse::InternalServerError().message(format!("Database error: {}", e))
//     })? {
//         Some(room) => {
//             if room.can_enter() {
//                 Ok(Json(room))
//             } else {
//                 Err(HttpResponse::Forbidden().message("Room cannot be entered"))
//             }
//         }
//         None => {
//             // 如果房间不存在，创建一个新的
//             let new_room = Room::new(name, None);
//             let created_room = repository.create(&new_room).await.map_err(|e| {
//                 HttpResponse::InternalServerError().message(format!("Failed to create room: {}", e))
//             })?;
//             Ok(Json(created_room))
//         }
//     }
// }

// /// 删除房间
// #[utoipa::path(
//     delete,
//     path = "/api/v1/rooms/{name}",
//     params(
//         ("name" = String, Path, description = "房间名称")
//     ),
//     responses(
//         (status = 200, description = "房间删除成功"),
//         (status = 404, description = "房间不存在"),
//         (status = 500, description = "服务器内部错误")
//     ),
//     tag = "rooms"
// )]
// pub async fn delete(
//     Path(name): Path<String>,
//     State(pool): State<Arc<DbPool>>,
// ) -> impl IntoResponse {
//     if name.is_empty() {
//         return HttpResponse::BadRequest()
//             .message("Invalid room name")
//             .into_response();
//     }

//     let repository = SqliteRoomRepository::new(pool.clone());

//     match repository.delete(&name).await.map_err(|e| {
//         HttpResponse::InternalServerError().message(format!("Failed to delete room: {}", e))
//     }) {
//         Ok(true) => HttpResponse::Ok()
//             .message("Room deleted successfully")
//             .into_response(),
//         Ok(false) => HttpResponse::NotFound()
//             .message("Room not found")
//             .into_response(),
//         Err(e) => e.into_response(),
//     }
// }
