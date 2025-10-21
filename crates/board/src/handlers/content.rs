use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::Response;
use axum_responses::http::HttpResponse;
use chrono::{Duration as ChronoDuration, NaiveDateTime};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use tokio_util::io::ReaderStream;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::{
    UploadFileDescriptor,
    content::{ContentType, RoomContent},
};
use crate::repository::{
    IRoomContentRepository, IRoomRepository, IRoomUploadReservationRepository,
    SqliteRoomContentRepository, SqliteRoomRepository, SqliteRoomUploadReservationRepository,
};
use crate::services::RoomTokenClaims;
use crate::state::AppState;

use super::{TokenQuery, verify_room_token};

const STORAGE_ROOT: &str = "storage/rooms";
const UPLOAD_RESERVATION_TTL_SECONDS: i64 = 10;

type HandlerResult<T> = Result<Json<T>, HttpResponse>;

#[derive(Debug, Serialize, ToSchema)]
pub struct RoomContentView {
    pub id: i64,
    pub content_type: ContentType,
    pub file_name: Option<String>,
    pub url: Option<String>,
    pub size: Option<i64>,
    pub mime_type: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<RoomContent> for RoomContentView {
    fn from(value: RoomContent) -> Self {
        let file_name = value.path.as_ref().and_then(|path| {
            Path::new(path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
        });
        Self {
            id: value.id.unwrap_or_default(),
            content_type: value.content_type,
            file_name,
            url: value.url,
            size: value.size,
            mime_type: value.mime_type,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadContentResponse {
    pub uploaded: Vec<RoomContentView>,
    pub current_size: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadPreparationRequest {
    pub files: Vec<UploadFileDescriptor>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadPreparationResponse {
    pub reservation_id: i64,
    pub reserved_size: i64,
    pub expires_at: NaiveDateTime,
    pub current_size: i64,
    pub remaining_size: i64,
    pub max_size: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadContentQuery {
    pub token: String,
    pub reservation_id: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteContentRequest {
    pub ids: Vec<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteContentResponse {
    pub deleted: Vec<i64>,
    pub freed_size: i64,
    pub current_size: i64,
}

#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}/contents",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    responses(
        (status = 200, description = "房间文件列表", body = [RoomContentView]),
        (status = 401, description = "token 无效"),
        (status = 404, description = "房间不存在")
    ),
    tag = "content"
)]
pub async fn list_contents(
    AxumPath(name): AxumPath<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<RoomContentView>> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    let room_id = room_id_or_error(&verified.claims)?;

    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    let repository = SqliteRoomContentRepository::new(app_state.db_pool.clone());
    let contents = repository.list_by_room(room_id).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to list contents: {e}"))
    })?;

    Ok(Json(
        contents.into_iter().map(RoomContentView::from).collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/contents/prepare",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = UploadPreparationRequest,
    responses(
        (status = 200, description = "预留上传空间成功", body = UploadPreparationResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无上传权限"),
        (status = 404, description = "房间不存在"),
        (status = 413, description = "超出房间容量限制")
    ),
    tag = "content"
)]
pub async fn prepare_upload(
    AxumPath(name): AxumPath<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UploadPreparationRequest>,
) -> HandlerResult<UploadPreparationResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }
    if payload.files.is_empty() {
        return Err(HttpResponse::BadRequest().message("No files provided"));
    }

    let mut verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    let mut total_size: i64 = 0;
    let mut names = HashSet::new();
    for file in &payload.files {
        if file.size <= 0 {
            return Err(
                HttpResponse::BadRequest().message(format!("Invalid file size for {}", file.name))
            );
        }
        if !names.insert(file.name.clone()) {
            return Err(
                HttpResponse::BadRequest().message(format!("Duplicate file name {}", file.name))
            );
        }
        total_size = total_size
            .checked_add(file.size)
            .ok_or_else(|| HttpResponse::BadRequest().message("Total size overflow"))?;
    }

    let manifest_json = serde_json::to_string(&payload.files).map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Serialize manifest failed: {e}"))
    })?;

    let reservation_repo = SqliteRoomUploadReservationRepository::new(app_state.db_pool.clone());
    let ttl = ChronoDuration::seconds(UPLOAD_RESERVATION_TTL_SECONDS);

    let (reservation, updated_room) = reservation_repo
        .reserve_upload(
            &verified.room,
            &verified.claims.jti,
            &manifest_json,
            total_size,
            ttl,
        )
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.to_lowercase().contains("limit exceeded") {
                HttpResponse::PayloadTooLarge().message("Room size limit exceeded")
            } else {
                HttpResponse::InternalServerError().message(format!("Reserve upload failed: {msg}"))
            }
        })?;

    let reservation_id = reservation
        .id
        .ok_or_else(|| HttpResponse::InternalServerError().message("Reservation id missing"))?;

    verified.room = updated_room.clone();

    let db_pool = app_state.db_pool.clone();
    tokio::spawn(async move {
        sleep(StdDuration::from_secs(
            UPLOAD_RESERVATION_TTL_SECONDS as u64,
        ))
        .await;
        let repo = SqliteRoomUploadReservationRepository::new(db_pool);
        if let Err(err) = repo.release_if_pending(reservation_id).await {
            log::warn!(
                "Failed to release expired reservation {}: {}",
                reservation_id,
                err
            );
        }
    });

    let remaining_size = (updated_room.max_size - updated_room.current_size).max(0);

    Ok(Json(UploadPreparationResponse {
        reservation_id,
        reserved_size: reservation.reserved_size,
        expires_at: reservation.expires_at,
        current_size: updated_room.current_size,
        remaining_size,
        max_size: updated_room.max_size,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/contents",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token"),
        ("reservation_id" = i64, Query, description = "上传预留 ID")
    ),
    responses(
        (status = 200, description = "上传成功", body = UploadContentResponse),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无上传权限"),
        (status = 404, description = "房间不存在"),
        (status = 413, description = "超出房间容量限制")
    ),
    tag = "content"
)]
pub async fn upload_contents(
    AxumPath(name): AxumPath<String>,
    Query(query): Query<UploadContentQuery>,
    State(app_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> HandlerResult<UploadContentResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }
    if query.reservation_id <= 0 {
        return Err(HttpResponse::BadRequest().message("Invalid reservation id"));
    }

    let mut verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let reservation_repo = SqliteRoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation = reservation_repo
        .fetch_by_id(query.reservation_id)
        .await
        .map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Load reservation failed: {e}"))
        })?
        .ok_or_else(|| HttpResponse::BadRequest().message("Reservation not found"))?;

    if reservation.room_id != room_id {
        return Err(HttpResponse::Forbidden().message("Reservation not for this room"));
    }
    if reservation.token_jti != verified.claims.jti {
        return Err(HttpResponse::Forbidden().message("Reservation token mismatch"));
    }

    let now = chrono::Utc::now().naive_utc();
    if reservation.expires_at < now {
        reservation_repo
            .release_if_pending(query.reservation_id)
            .await
            .ok();
        return Err(HttpResponse::BadRequest().message("Reservation expired"));
    }

    let expected_files: Vec<UploadFileDescriptor> =
        serde_json::from_str(&reservation.file_manifest).map_err(|e| {
            HttpResponse::InternalServerError()
                .message(format!("Parse reservation manifest failed: {e}"))
        })?;
    if expected_files.is_empty() {
        return Err(HttpResponse::BadRequest().message("Reservation manifest empty"));
    }

    let mut expected_map = HashMap::new();
    for file in expected_files {
        if expected_map.insert(file.name.clone(), file).is_some() {
            return Err(HttpResponse::InternalServerError()
                .message("Reservation manifest has duplicate file names"));
        }
    }

    let storage_dir = ensure_room_storage(&verified.room.slug)
        .await
        .map_err(|e| {
            HttpResponse::InternalServerError()
                .message(format!("Failed to prepare storage directory: {e}"))
        })?;

    struct TempUpload {
        name: String,
        path: PathBuf,
        size: i64,
        mime: Option<String>,
    }

    let mut staged: Vec<TempUpload> = Vec::new();
    let mut seen = HashSet::new();

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| HttpResponse::BadRequest().message(format!("Invalid multipart data: {e}")))?
    {
        let file_name = field
            .file_name()
            .map(|name| name.to_string())
            .ok_or_else(|| HttpResponse::BadRequest().message("File name missing"))?;

        let expected = if let Some(exp) = expected_map.get(&file_name) {
            exp
        } else {
            for temp in &staged {
                fs::remove_file(&temp.path).await.ok();
            }
            return Err(HttpResponse::BadRequest().message(format!("Unexpected file: {file_name}")));
        };

        if !seen.insert(file_name.clone()) {
            for temp in &staged {
                fs::remove_file(&temp.path).await.ok();
            }
            return Err(
                HttpResponse::BadRequest().message(format!("Duplicate upload file: {file_name}"))
            );
        }

        let safe_file_name = sanitize_filename::sanitize(&file_name);
        let unique_segment = Uuid::new_v4().to_string();
        let file_path = storage_dir.join(format!("{unique_segment}_{safe_file_name}"));
        let mut temp_file = fs::File::create(&file_path).await.map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Cannot create file: {e}"))
        })?;

        let mut size: i64 = 0;
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| {
                HttpResponse::BadRequest().message(format!("Read upload chunk failed: {e}"))
            })?;
            size += chunk.len() as i64;
            temp_file.write_all(&chunk).await.map_err(|e| {
                HttpResponse::InternalServerError().message(format!("Write file failed: {e}"))
            })?;
        }
        temp_file.flush().await.map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Flush file failed: {e}"))
        })?;

        if size != expected.size {
            fs::remove_file(&file_path).await.ok();
            for temp in &staged {
                fs::remove_file(&temp.path).await.ok();
            }
            return Err(
                HttpResponse::BadRequest().message(format!("File size mismatch for {file_name}"))
            );
        }

        let mime = mime_guess::from_path(&file_path)
            .first_raw()
            .map(|m| m.to_string());

        staged.push(TempUpload {
            name: file_name,
            path: file_path,
            size,
            mime,
        });
    }

    if staged.is_empty() {
        return Err(HttpResponse::BadRequest().message("No files uploaded"));
    }

    if staged.len() != expected_map.len() {
        for temp in &staged {
            fs::remove_file(&temp.path).await.ok();
        }
        return Err(HttpResponse::BadRequest().message("Uploaded file count mismatch reservation"));
    }

    let repository = SqliteRoomContentRepository::new(app_state.db_pool.clone());
    let mut uploaded = Vec::new();
    let mut actual_total: i64 = 0;

    for temp in &staged {
        let now = chrono::Utc::now().naive_utc();
        let mut content = RoomContent {
            id: None,
            room_id,
            content_type: ContentType::File,
            text: None,
            url: None,
            path: None,
            size: None,
            mime_type: None,
            created_at: now,
            updated_at: now,
        };
        content.set_path(
            temp.path.to_string_lossy().to_string(),
            ContentType::File,
            temp.size,
            temp.mime
                .clone()
                .unwrap_or_else(|| "application/octet-stream".to_string()),
        );

        let saved = match repository.create(&content).await {
            Ok(value) => value,
            Err(e) => {
                for item in &staged {
                    if let Err(err) = fs::remove_file(&item.path).await {
                        log::warn!(
                            "Failed to remove temp file {}: {}",
                            item.path.display(),
                            err
                        );
                    }
                }
                return Err(HttpResponse::InternalServerError()
                    .message(format!("Persist content failed: {e}")));
            }
        };

        actual_total = actual_total
            .checked_add(temp.size)
            .ok_or_else(|| HttpResponse::InternalServerError().message("Total size overflow"))?;
        uploaded.push(RoomContentView::from(saved));
    }

    let actual_manifest: Vec<UploadFileDescriptor> = staged
        .iter()
        .map(|temp| UploadFileDescriptor {
            name: temp.name.clone(),
            size: temp.size,
            mime: temp.mime.clone(),
        })
        .collect();
    let actual_manifest_json = serde_json::to_string(&actual_manifest).map_err(|e| {
        HttpResponse::InternalServerError()
            .message(format!("Serialize actual manifest failed: {e}"))
    })?;

    let updated_room = reservation_repo
        .consume_reservation(
            query.reservation_id,
            room_id,
            &verified.claims.jti,
            actual_total,
            &actual_manifest_json,
        )
        .await
        .map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Finalize reservation failed: {e}"))
        })?;

    verified.room = updated_room;

    Ok(Json(UploadContentResponse {
        uploaded,
        current_size: verified.room.current_size,
    }))
}

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
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<DeleteContentRequest>,
) -> HandlerResult<DeleteContentResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    if payload.ids.is_empty() {
        return Err(HttpResponse::BadRequest().message("No content id provided"));
    }

    let mut verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_delete(),
        ContentPermission::Delete,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let repository = SqliteRoomContentRepository::new(app_state.db_pool.clone());
    let existing_contents = repository
        .list_by_room(room_id)
        .await
        .map_err(|e| HttpResponse::InternalServerError().message(format!("Query failed: {e}")))?;

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
        return Err(HttpResponse::NotFound().message("Contents not found"));
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
        .map_err(|e| HttpResponse::InternalServerError().message(format!("Delete failed: {e}")))?;

    if freed_size > 0 {
        verified.room.current_size = (verified.room.current_size - freed_size).max(0);
        let room_repo = SqliteRoomRepository::new(app_state.db_pool.clone());
        verified.room = room_repo.update(&verified.room).await.map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Update room failed: {e}"))
        })?;
    }

    Ok(Json(DeleteContentResponse {
        deleted: ids,
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
pub async fn download_content(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Response, HttpResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let repository = SqliteRoomContentRepository::new(app_state.db_pool.clone());
    let content = repository
        .find_by_id(content_id)
        .await
        .map_err(|e| HttpResponse::InternalServerError().message(format!("Query failed: {e}")))?
        .ok_or_else(|| HttpResponse::NotFound().message("Content not found"))?;

    if content.room_id != room_id {
        return Err(HttpResponse::Forbidden().message("Content not in room"));
    }

    let path = content
        .path
        .ok_or_else(|| HttpResponse::NotFound().message("Content not stored on disk"))?;

    let file = fs::File::open(&path)
        .await
        .map_err(|_| HttpResponse::NotFound().message("File missing on disk"))?;

    let file_name = Path::new(&path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "download.bin".to_string());

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let mut response = Response::new(body);
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
        .map_err(|_| {
            HttpResponse::InternalServerError().message("Failed to build response headers")
        })?;
    response
        .headers_mut()
        .insert(CONTENT_DISPOSITION, disposition);

    if let Some(mime) = content.mime_type
        && let Ok(value) = HeaderValue::from_str(&mime)
    {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }

    Ok(response)
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
) -> Result<(), HttpResponse> {
    if !room_allows {
        return Err(HttpResponse::Forbidden().message("Permission denied by room"));
    }
    let permission = claims.as_permission();
    let token_allows = match action {
        ContentPermission::View => permission.can_view(),
        ContentPermission::Edit => permission.can_edit(),
        ContentPermission::Delete => permission.can_delete(),
    };
    if !token_allows {
        return Err(HttpResponse::Forbidden().message("Permission denied by token"));
    }
    Ok(())
}

async fn ensure_room_storage(room_name: &str) -> Result<PathBuf, std::io::Error> {
    let safe = sanitize_filename::sanitize(room_name);
    let dir = PathBuf::from(STORAGE_ROOT).join(safe);
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}

fn room_id_or_error(claims: &RoomTokenClaims) -> Result<i64, HttpResponse> {
    if claims.room_id <= 0 {
        return Err(HttpResponse::InternalServerError().message("Room id missing in claims"));
    }
    Ok(claims.room_id)
}
