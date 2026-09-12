use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, Path as AxumPath, Query, State, multipart::Field};
use futures::StreamExt;
use serde::Deserialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use utoipa::ToSchema;

use crate::dto::content::{
    PresignedUpload, RoomContentView, UploadContentResponse, UploadPreparationRequest,
    UploadPreparationResponse,
};
use crate::errors::AppError;
use crate::models::{
    UploadFileDescriptor,
    content::{ContentType, RoomContent},
};
use crate::repository::{
    IRoomContentBlobRepository, IRoomContentRepository, IRoomRepository,
    IRoomUploadReservationRepository, RoomContentBlob, RoomContentBlobRepository,
    RoomContentRepository, RoomRepository, RoomUploadReservationRepository,
};
use crate::state::AppState;
use crate::validation::RoomNameValidator;

use super::{HandlerResult, room_id_or_error};
use crate::authz::{Authz, Resource};
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::room::role::Capability;
use crate::models::room::upload_file_policy::{
    is_safe_upload_file_name, upload_file_type_violation,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadReservationQuery {
    pub reservation_id: i64,
}

pub(super) struct TempUpload {
    pub(super) original_name: String,
    pub(super) path: PathBuf,
    pub(super) size: i64,
    pub(super) mime: Option<String>,
    /// 落盘时流式计算的内容 SHA-256（小写 hex）
    pub(super) hash: String,
}

/// 预留上传失败的统一映射：容量超限 → 413，其余 → 500。
pub(super) fn map_reservation_error(e: anyhow::Error) -> AppError {
    let msg = e.to_string();
    if msg.to_lowercase().contains("limit exceeded") {
        AppError::payload_too_large("Room size limit exceeded")
    } else {
        AppError::internal(format!("Reserve upload failed: {msg}"))
    }
}

/// 内容寻址落盘（per-room 去重）：命中既有 blob 则引用 +1 复用对象，
/// 未命中则写入 `{room_id}/{hash}` 并建 blob 行（原子 upsert 防并发重复）。
pub(crate) async fn store_content_deduped(
    app_state: &Arc<AppState>,
    blob_repo: &dyn IRoomContentBlobRepository,
    room_id: i64,
    hash: String,
    temp_path: &Path,
    size: i64,
) -> Result<String, AppError> {
    if let Some(blob) = blob_repo
        .find_by_hash(room_id, &hash)
        .await
        .map_err(|e| AppError::internal(format!("Blob lookup failed: {e}")))?
    {
        blob_repo
            .upsert_ref(RoomContentBlob {
                id: None,
                room_id,
                hash: blob.hash.clone(),
                locator: blob.locator.clone(),
                size: blob.size,
                ref_count: blob.ref_count,
            })
            .await
            .map_err(|e| AppError::internal(format!("Blob ref update failed: {e}")))?;
        return Ok(blob.locator);
    }

    let key = format!("{room_id}/{hash}");
    let locator = app_state
        .storage
        .store_file(&key, temp_path)
        .await
        .map_err(|e| AppError::internal(format!("Store content failed: {e}")))?;
    let blob = blob_repo
        .upsert_ref(RoomContentBlob {
            id: None,
            room_id,
            hash,
            locator: locator.clone(),
            size,
            ref_count: 1,
        })
        .await
        .map_err(|e| AppError::internal(format!("Blob register failed: {e}")))?;
    Ok(blob.locator)
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
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = room_id_or_error(&verified.claims)?;
    authz.require(Capability::FileList, &Resource::Room { room_id })?;

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let contents = repository
        .list_by_room(room_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list contents: {e}")))?;

    // 隐藏文件仅对拥有 file.visibility.manage 能力的身份可见；own 作用域按创建者判定。
    let views = contents
        .into_iter()
        .filter(|content| {
            !content.hidden
                || authz.permits(
                    Capability::FileVisibilityManage,
                    &Resource::Content {
                        room_id,
                        content_type: content.content_type,
                        created_by_jti: content.created_by_jti.as_deref(),
                    },
                )
        })
        .map(RoomContentView::from)
        .collect();

    Ok(Json(views))
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
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = room_id_or_error(&verified.claims)?;
    authz.require(Capability::FileUpload, &Resource::Room { room_id })?;

    let mut total_size: i64 = 0;
    let mut names = HashSet::new();
    for file in &payload.files {
        if file.size <= 0 {
            return Err(AppError::validation(format!(
                "Invalid file size for {}",
                file.name
            )));
        }
        if !is_safe_upload_file_name(&file.name) {
            return Err(AppError::validation(format!(
                "Invalid file name: {}",
                file.name
            )));
        }
        if !names.insert(file.name.clone()) {
            return Err(AppError::validation(format!(
                "Duplicate file name {}",
                file.name
            )));
        }
        if !verified.room.upload_file_type.permits(&file.name) {
            return Err(AppError::validation(upload_file_type_violation(&file.name)));
        }
        total_size = total_size
            .checked_add(file.size)
            .ok_or_else(|| AppError::validation("Total size overflow"))?;
    }

    // 秒传（per-room 去重）：携带的整文件哈希与大小都命中既有 blob 时，
    // 跳过传输直接建内容记录；命中部分不计入预留。
    let blob_repo = RoomContentBlobRepository::new(app_state.db_pool.clone());
    let content_repo = RoomContentRepository::new(app_state.db_pool.clone());
    let mut instant_uploads = Vec::new();
    let mut instant_total: i64 = 0;
    let mut pending_files = Vec::new();
    for file in payload.files {
        let hit = match (&file.file_hash, file.size > 0) {
            (Some(hash), true) => blob_repo
                .find_by_hash(room_id, hash)
                .await
                .map_err(|e| AppError::internal(format!("Blob lookup failed: {e}")))?
                .filter(|blob| blob.size == file.size),
            _ => None,
        };
        if let Some(blob) = hit {
            blob_repo
                .upsert_ref(RoomContentBlob {
                    id: None,
                    room_id,
                    hash: blob.hash.clone(),
                    locator: blob.locator.clone(),
                    size: blob.size,
                    ref_count: blob.ref_count,
                })
                .await
                .map_err(|e| AppError::internal(format!("Blob ref update failed: {e}")))?;

            let mut content = RoomContent {
                id: None,
                room_id,
                content_type: ContentType::File,
                text: None,
                url: None,
                path: None,
                hash: None,
                file_name: Some(file.name.clone()),
                size: None,
                mime_type: None,
                sequence_number: 0,
                created_by_jti: Some(verified.claims.jti.clone()),
                hidden: false,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            };
            content.set_path(
                blob.locator.clone(),
                ContentType::File,
                file.size,
                file.mime
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
            );
            content.hash = Some(blob.hash);
            let saved = content_repo
                .create(&content)
                .await
                .map_err(|e| AppError::internal(format!("Persist content failed: {e}")))?;
            broadcast_content_created(app_state.clone(), name.clone(), saved.clone());
            instant_uploads.push(RoomContentView::from(saved));
            instant_total = instant_total
                .checked_add(file.size)
                .ok_or_else(|| AppError::validation("Total size overflow"))?;
        } else {
            pending_files.push(file);
        }
    }

    if instant_total > 0 {
        if !verified.room.can_add_content(instant_total) {
            return Err(AppError::payload_too_large("Room size limit exceeded"));
        }
        verified.room.current_size += instant_total;
        let room_repo = RoomRepository::new(app_state.db_pool.clone());
        verified.room = room_repo
            .update(&verified.room)
            .await
            .map_err(|e| AppError::internal(format!("Update room failed: {e}")))?;
    }

    // presigned 模式：服务端为每个文件生成唯一对象 key 写入清单，
    // 预留成功后逐文件签发直传 URL；proxy 模式保持原语义。
    let presigned = app_state.transfer_mode() == crate::config::TransferMode::Presigned;
    let mut files = pending_files;
    if presigned {
        for file in &mut files {
            file.storage_key = Some(crate::storage::presigned_key(room_id, &file.name));
        }
    }

    let reservation_repo = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let ttl = app_state.upload_reservation_ttl();

    // 待传文件为空（全部秒传命中）时无需预留；expires_at 仅作占位回显。
    let (reservation_id, reserved_size, expires_at, updated_room) = if files.is_empty() {
        (None, 0, verified.room.updated_at, verified.room.clone())
    } else {
        let manifest_json = serde_json::to_string(&files)
            .map_err(|e| AppError::internal(format!("Serialize manifest failed: {e}")))?;
        let mut total_size: i64 = 0;
        for file in &files {
            total_size = total_size
                .checked_add(file.size)
                .ok_or_else(|| AppError::validation("Total size overflow"))?;
        }
        let (reservation, updated_room) = reservation_repo
            .reserve_upload(
                &verified.room,
                &verified.claims.jti,
                &verified.claims.jti,
                &manifest_json,
                total_size,
                ttl,
            )
            .await
            .map_err(map_reservation_error)?;
        verified.room = updated_room.clone();
        (
            reservation.id,
            reservation.reserved_size,
            reservation.expires_at,
            updated_room,
        )
    };

    let remaining_size = (updated_room.max_size - updated_room.current_size).max(0);

    let presigned_uploads = if presigned && !files.is_empty() {
        let ttl = app_state.presign_ttl();
        let expires_at =
            chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl.as_secs() as i64);
        let mut uploads = Vec::new();
        for file in &files {
            let key = file
                .storage_key
                .clone()
                .ok_or_else(|| AppError::internal("Presigned manifest is missing storage key"))?;
            let url = app_state
                .storage
                .presign_write(&key, ttl)
                .await
                .map_err(|e| AppError::internal(format!("Presign upload failed: {e}")))?;
            uploads.push(PresignedUpload {
                file_name: file.name.clone(),
                method: "PUT".to_string(),
                url,
                expires_at,
            });
        }
        Some(uploads)
    } else {
        None
    };

    Ok(Json(UploadPreparationResponse {
        reservation_id,
        reserved_size,
        expires_at,
        current_size: updated_room.current_size,
        remaining_size,
        max_size: updated_room.max_size,
        presigned_uploads,
        instant_uploads: (!instant_uploads.is_empty()).then_some(instant_uploads),
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
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    let room_id = room_id_or_error(&verified.claims)?;
    authz.require(Capability::FileUpload, &Resource::Room { room_id })?;

    let reservation_repo = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation = reservation_repo
        .fetch_by_id(query.reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("Load reservation failed: {e}")))?
        .ok_or_else(|| AppError::validation("Reservation not found"))?;

    if reservation.room_id != room_id {
        return Err(AppError::permission_denied("Reservation not for this room"));
    }
    if reservation.owner_token_jti != verified.claims.jti {
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

    let scratch_dir =
        crate::chunk_temp_storage::reservation_dir(app_state.storage_root(), query.reservation_id);
    tokio::fs::create_dir_all(&scratch_dir)
        .await
        .map_err(|e| AppError::internal(format!("Failed to prepare storage directory: {e}")))?;

    let staged = stage_multipart_uploads(multipart, &expected_map, &scratch_dir).await?;

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let (uploaded, actual_total) = persist_staged_uploads(
        &repository,
        &app_state,
        &name,
        room_id,
        &verified.claims.jti,
        &staged,
    )
    .await?;
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

    let file_path = storage_dir.join(format!("stage_{}", uuid::Uuid::new_v4()));
    let (size, hash) = write_field_to_file(&mut field, &file_path).await?;

    if size != expected.size {
        fs::remove_file(&file_path).await.ok();
        return Err(AppError::validation(format!(
            "File size mismatch for {file_name}"
        )));
    }

    // mime 按原始文件名推断；暂存文件名是随机的，不含扩展名信息。
    let mime = mime_guess::from_path(&file_name)
        .first_raw()
        .map(|m| m.to_string());

    Ok(TempUpload {
        original_name: file_name,
        path: file_path,
        size,
        mime,
        hash,
    })
}

pub(super) async fn write_field_to_file(
    field: &mut Field<'_>,
    file_path: &Path,
) -> Result<(i64, String), AppError> {
    let mut temp_file = fs::File::create(file_path)
        .await
        .map_err(|e| AppError::internal(format!("Cannot create file: {e}")))?;

    let mut size: i64 = 0;
    let mut hasher = sha2::Sha256::new();
    while let Some(chunk) = field.next().await {
        let chunk =
            chunk.map_err(|e| AppError::validation(format!("Read upload chunk failed: {e}")))?;
        size += chunk.len() as i64;
        hasher.update(&chunk);
        temp_file
            .write_all(&chunk)
            .await
            .map_err(|e| AppError::internal(format!("Write file failed: {e}")))?;
    }
    temp_file
        .flush()
        .await
        .map_err(|e| AppError::internal(format!("Flush file failed: {e}")))?;

    use sha2::Digest;
    Ok((size, hex::encode(hasher.finalize())))
}

pub(super) async fn persist_staged_uploads(
    repository: &RoomContentRepository,
    app_state: &Arc<AppState>,
    room_name: &str,
    room_id: i64,
    owner_jti: &str,
    staged: &[TempUpload],
) -> Result<(Vec<RoomContentView>, i64), AppError> {
    let blob_repo = RoomContentBlobRepository::new(app_state.db_pool.clone());
    let mut uploaded = Vec::new();
    let mut actual_total: i64 = 0;

    for temp in staged {
        let locator = match store_content_deduped(
            app_state,
            &blob_repo,
            room_id,
            temp.hash.clone(),
            &temp.path,
            temp.size,
        )
        .await
        {
            Ok(locator) => locator,
            Err(e) => {
                cleanup_staged_uploads(staged).await;
                return Err(e);
            }
        };

        let saved = match repository
            .create(&build_file_content(
                room_id, owner_jti, temp, locator, &temp.hash,
            ))
            .await
        {
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

fn build_file_content(
    room_id: i64,
    owner_jti: &str,
    temp: &TempUpload,
    locator: String,
    hash: &str,
) -> RoomContent {
    let now = chrono::Utc::now().naive_utc();
    let mut content = RoomContent {
        id: None,
        room_id,
        content_type: ContentType::File,
        text: None,
        url: None,
        path: None,
        hash: None,
        file_name: Some(temp.original_name.clone()),
        size: None,
        mime_type: None,
        sequence_number: 0,
        created_by_jti: Some(owner_jti.to_string()),
        hidden: false,
        created_at: now,
        updated_at: now,
    };
    content.set_path(
        locator,
        ContentType::File,
        temp.size,
        temp.mime
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string()),
    );
    content.hash = Some(hash.to_string());
    content
}

pub(super) fn broadcast_content_created(
    app_state: Arc<AppState>,
    room_name: String,
    content: RoomContent,
) {
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

pub(super) async fn consume_upload_reservation(
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
            storage_key: None,
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
