use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use axum::Json;
use axum::extract::{Multipart, Path as AxumPath, Query, State, multipart::Field};
use futures::StreamExt;
use serde::Deserialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use utoipa::ToSchema;

use crate::dto::content::{
    RoomContentView, UploadContentResponse, UploadPreparationRequest, UploadPreparationResponse,
};
use crate::errors::AppError;
use crate::models::{
    UploadFileDescriptor,
    content::{ContentType, RoomContent},
};
use crate::repository::{
    IRoomContentRepository, IRoomUploadReservationRepository, RoomContentRepository,
    RoomUploadReservationRepository,
};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::{
    ContentPermission, HandlerResult, ensure_permission, ensure_room_storage, room_id_or_error,
};
use crate::handlers::{AuthToken, verify_room_token};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadReservationQuery {
    pub reservation_id: i64,
}

struct TempUpload {
    original_name: String,
    path: PathBuf,
    size: i64,
    mime: Option<String>,
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
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<RoomContentView>> {
    // Validate room name using the new validation framework
    RoomNameValidator::validate_identifier(&name)?;

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    let room_id = room_id_or_error(&verified.claims)?;

    // Manual permission check is used for now
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let contents = repository
        .list_by_room(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list contents: {e}")))?;

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
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UploadPreparationRequest>,
) -> HandlerResult<UploadPreparationResponse> {
    // Validate room name using the new validation framework
    RoomNameValidator::validate_identifier(&name)?;

    if payload.files.is_empty() {
        return Err(AppError::validation("No files provided"));
    }

    let mut verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    let mut total_size: i64 = 0;
    let mut names = HashSet::new();
    for file in &payload.files {
        if file.size <= 0 {
            return Err(AppError::validation(format!(
                "Invalid file size for {}",
                file.name
            )));
        }
        if !names.insert(file.name.clone()) {
            return Err(AppError::validation(format!(
                "Duplicate file name {}",
                file.name
            )));
        }
        total_size = total_size
            .checked_add(file.size)
            .ok_or_else(|| AppError::validation("Total size overflow"))?;
    }

    let manifest_json = serde_json::to_string(&payload.files)
        .map_err(|e| AppError::internal(format!("Serialize manifest failed: {e}")))?;

    let reservation_repo = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let ttl = app_state.upload_reservation_ttl();

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
                AppError::payload_too_large("Room size limit exceeded")
            } else {
                AppError::internal(format!("Reserve upload failed: {msg}"))
            }
        })?;

    let reservation_id = reservation
        .id
        .ok_or_else(|| AppError::internal("Reservation id missing"))?;

    verified.room = updated_room.clone();

    let db_pool = app_state.db_pool.clone();
    let cleanup_delay_seconds = ttl.num_seconds().max(1) as u64;
    tokio::spawn(async move {
        sleep(StdDuration::from_secs(cleanup_delay_seconds)).await;
        let repo = RoomUploadReservationRepository::new(db_pool);
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
    AuthToken(token): AuthToken,
    Query(query): Query<UploadReservationQuery>,
    State(app_state): State<Arc<AppState>>,
    multipart: Multipart,
) -> HandlerResult<UploadContentResponse> {
    // Validate room name using the new validation framework
    RoomNameValidator::validate_identifier(&name)?;

    if query.reservation_id <= 0 {
        return Err(AppError::validation("Invalid reservation id"));
    }

    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;
    let reservation_repo = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation = reservation_repo
        .fetch_by_id(query.reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("Load reservation failed: {e}")))?
        .ok_or_else(|| AppError::validation("Reservation not found"))?;

    if reservation.room_id != room_id {
        return Err(AppError::permission_denied("Reservation not for this room"));
    }
    if reservation.token_jti != verified.claims.jti {
        return Err(AppError::permission_denied("Reservation token mismatch"));
    }

    let now = chrono::Utc::now().naive_utc();
    if reservation.expires_at < now {
        reservation_repo
            .release_if_pending(query.reservation_id)
            .await
            .ok();
        return Err(AppError::validation("Reservation expired"));
    }

    let expected_files: Vec<UploadFileDescriptor> =
        serde_json::from_str(&reservation.file_manifest)
            .map_err(|e| AppError::internal(format!("Parse reservation manifest failed: {e}")))?;
    if expected_files.is_empty() {
        return Err(AppError::validation("Reservation manifest empty"));
    }

    let expected_map = build_expected_manifest(expected_files)?;

    let storage_dir = ensure_room_storage(app_state.storage_root().as_ref(), room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to prepare storage directory: {e}")))?;

    let staged = stage_multipart_uploads(multipart, &expected_map, &storage_dir).await?;

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let (uploaded, actual_total) =
        persist_staged_uploads(&repository, &app_state, &name, room_id, &staged).await?;
    let current_size = consume_upload_reservation(
        &reservation_repo,
        query.reservation_id,
        room_id,
        &verified.claims.jti,
        actual_total,
        &staged,
    )
    .await?;

    Ok(Json(UploadContentResponse {
        uploaded,
        current_size,
    }))
}

fn build_expected_manifest(
    expected_files: Vec<UploadFileDescriptor>,
) -> Result<HashMap<String, UploadFileDescriptor>, AppError> {
    let mut expected_map = HashMap::new();
    for file in expected_files {
        if expected_map.insert(file.name.clone(), file).is_some() {
            return Err(AppError::internal(
                "Reservation manifest has duplicate file names",
            ));
        }
    }
    Ok(expected_map)
}

async fn stage_multipart_uploads(
    mut multipart: Multipart,
    expected_map: &HashMap<String, UploadFileDescriptor>,
    storage_dir: &Path,
) -> Result<Vec<TempUpload>, AppError> {
    let mut staged = Vec::new();
    let mut seen = HashSet::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(format!("Invalid multipart data: {e}")))?
    {
        match stage_upload_field(field, expected_map, storage_dir, &mut seen).await {
            Ok(temp_upload) => staged.push(temp_upload),
            Err(error) => {
                cleanup_staged_uploads(&staged).await;
                return Err(error);
            }
        }
    }

    if staged.is_empty() {
        return Err(AppError::validation("No files uploaded"));
    }

    if staged.len() != expected_map.len() {
        cleanup_staged_uploads(&staged).await;
        return Err(AppError::validation(
            "Uploaded file count mismatch reservation",
        ));
    }

    Ok(staged)
}

async fn stage_upload_field(
    mut field: Field<'_>,
    expected_map: &HashMap<String, UploadFileDescriptor>,
    storage_dir: &Path,
    seen: &mut HashSet<String>,
) -> Result<TempUpload, AppError> {
    let file_name = field
        .file_name()
        .map(|name| name.to_string())
        .ok_or_else(|| AppError::validation("File name missing"))?;

    let expected = expected_map
        .get(&file_name)
        .ok_or_else(|| AppError::validation(format!("Unexpected file: {file_name}")))?;

    if !seen.insert(file_name.clone()) {
        return Err(AppError::validation(format!(
            "Duplicate upload file: {file_name}"
        )));
    }

    let file_path = unique_upload_path(storage_dir, &file_name)?;
    let size = write_field_to_file(&mut field, &file_path).await?;

    if size != expected.size {
        fs::remove_file(&file_path).await.ok();
        return Err(AppError::validation(format!(
            "File size mismatch for {file_name}"
        )));
    }

    let mime = mime_guess::from_path(&file_path)
        .first_raw()
        .map(|m| m.to_string());

    Ok(TempUpload {
        original_name: file_name,
        path: file_path,
        size,
        mime,
    })
}

fn unique_upload_path(storage_dir: &Path, file_name: &str) -> Result<PathBuf, AppError> {
    let safe_file_name = sanitize_filename::sanitize(file_name);
    let mut final_filename = safe_file_name.clone();
    let mut counter = 1;
    let mut file_path = storage_dir.join(&final_filename);

    while file_path.exists() {
        let path = Path::new(&safe_file_name);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&safe_file_name);
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        final_filename = if extension.is_empty() {
            format!("{}({})", stem, counter)
        } else {
            format!("{}({}).{}", stem, counter, extension)
        };

        file_path = storage_dir.join(&final_filename);
        counter += 1;

        if counter > 1000 {
            return Err(AppError::internal("Too many files with the same name"));
        }
    }

    Ok(file_path)
}

async fn write_field_to_file(field: &mut Field<'_>, file_path: &Path) -> Result<i64, AppError> {
    let mut temp_file = fs::File::create(file_path)
        .await
        .map_err(|e| AppError::internal(format!("Cannot create file: {e}")))?;

    let mut size: i64 = 0;
    while let Some(chunk) = field.next().await {
        let chunk =
            chunk.map_err(|e| AppError::validation(format!("Read upload chunk failed: {e}")))?;
        size += chunk.len() as i64;
        temp_file
            .write_all(&chunk)
            .await
            .map_err(|e| AppError::internal(format!("Write file failed: {e}")))?;
    }
    temp_file
        .flush()
        .await
        .map_err(|e| AppError::internal(format!("Flush file failed: {e}")))?;

    Ok(size)
}

async fn persist_staged_uploads(
    repository: &RoomContentRepository,
    app_state: &Arc<AppState>,
    room_name: &str,
    room_id: i64,
    staged: &[TempUpload],
) -> Result<(Vec<RoomContentView>, i64), AppError> {
    let mut uploaded = Vec::new();
    let mut actual_total: i64 = 0;

    for temp in staged {
        let saved = match repository.create(&build_file_content(room_id, temp)).await {
            Ok(value) => value,
            Err(e) => {
                cleanup_staged_uploads(staged).await;
                return Err(AppError::internal(format!("Persist content failed: {e}")));
            }
        };

        actual_total = actual_total
            .checked_add(temp.size)
            .ok_or_else(|| AppError::internal("Total size overflow"))?;
        uploaded.push(RoomContentView::from(saved.clone()));
        broadcast_content_created(app_state.clone(), room_name.to_string(), saved);
    }

    Ok((uploaded, actual_total))
}

fn build_file_content(room_id: i64, temp: &TempUpload) -> RoomContent {
    let now = chrono::Utc::now().naive_utc();
    let mut content = RoomContent {
        id: None,
        room_id,
        content_type: ContentType::File,
        text: None,
        url: None,
        path: None,
        file_name: Some(temp.original_name.clone()),
        size: None,
        mime_type: None,
        sequence_number: 0,
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
    content
}

fn broadcast_content_created(app_state: Arc<AppState>, room_name: String, content: RoomContent) {
    let broadcaster = app_state.broadcaster.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcaster
            .broadcast_content_created(&room_name, &content)
            .await
        {
            log::warn!("Failed to broadcast content created event: {}", e);
        }
    });
}

async fn consume_upload_reservation(
    reservation_repo: &RoomUploadReservationRepository,
    reservation_id: i64,
    room_id: i64,
    token_jti: &str,
    actual_total: i64,
    staged: &[TempUpload],
) -> Result<i64, AppError> {
    let actual_manifest_json = serde_json::to_string(&actual_manifest(staged))
        .map_err(|e| AppError::internal(format!("Serialize actual manifest failed: {e}")))?;

    let updated_room = reservation_repo
        .consume_reservation(
            reservation_id,
            room_id,
            token_jti,
            actual_total,
            &actual_manifest_json,
        )
        .await
        .map_err(|e| AppError::internal(format!("Finalize reservation failed: {e}")))?;

    Ok(updated_room.current_size)
}

fn actual_manifest(staged: &[TempUpload]) -> Vec<UploadFileDescriptor> {
    staged
        .iter()
        .map(|temp| UploadFileDescriptor {
            name: temp.original_name.clone(),
            size: temp.size,
            mime: temp.mime.clone(),
            chunk_size: None,
            file_hash: None,
        })
        .collect()
}

async fn cleanup_staged_uploads(staged: &[TempUpload]) {
    for item in staged {
        if let Err(err) = fs::remove_file(&item.path).await {
            log::warn!(
                "Failed to remove temp file {}: {}",
                item.path.display(),
                err
            );
        }
    }
}
