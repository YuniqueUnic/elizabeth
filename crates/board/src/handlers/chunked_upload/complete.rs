use std::path::Path as StdPath;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};

use super::super::{AuthToken, verify_room_token};
use super::ensure_reservation_access;
use chrono::Utc;
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    dto::chunked_upload::{FileMergeRequest, FileMergeResponse, MergedFileInfo},
    errors::{AppError, AppResult},
    models::room::{
        chunk_upload::RoomChunkUpload,
        content::{ContentType, RoomContent},
        upload_reservation::{RoomUploadReservation, UploadFileDescriptor, UploadStatus},
    },
    repository::{
        IRoomContentRepository,
        room_chunk_upload_repository::{IRoomChunkUploadRepository, RoomChunkUploadRepository},
        room_content_repository::RoomContentRepository,
        room_upload_reservation_repository::{
            IRoomUploadReservationRepository, RoomUploadReservationRepository,
        },
    },
    state::AppState,
    validation::RoomNameValidator,
};

type HandlerResult<T> = AppResult<Json<T>>;

/// 完成文件合并
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/uploads/chunks/complete",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    request_body = FileMergeRequest,
    responses(
        (status = 200, description = "文件合并成功", body = FileMergeResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "房间不存在或预留记录不存在"),
        (status = 409, description = "文件合并冲突或状态不正确"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "chunked-upload"
)]
pub async fn complete_file_merge(
    Path(room_name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<FileMergeRequest>,
) -> HandlerResult<FileMergeResponse> {
    RoomNameValidator::validate_identifier(&room_name)?;

    let verified = verify_room_token(app_state.clone(), &room_name, &token).await?;
    let room = verified.room.clone();
    let room_id = room
        .id
        .ok_or_else(|| AppError::internal("房间 ID 不能为空"))?;
    let reservation_repository = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation_id = parse_reservation_id(&payload.reservation_id)?;
    let reservation = load_reservation(&reservation_repository, reservation_id).await?;
    validate_reservation_for_merge(&reservation, room_id)?;
    ensure_reservation_access(&reservation, &verified)?;

    let chunk_repository = RoomChunkUploadRepository::new(app_state.db_pool.clone());
    let reservation_db_id = reservation_id_or_error(&reservation)?;
    let sorted_chunks = load_uploaded_chunks(
        &chunk_repository,
        reservation_db_id,
        reservation.total_chunks,
    )
    .await?;
    let temp_dir = format!("/tmp/elizabeth/chunks/{}/", payload.reservation_id);
    let final_file_path = format!("{}/merged_file", temp_dir);

    merge_and_verify_chunks(
        &sorted_chunks,
        &final_file_path,
        &payload.final_hash,
        &reservation_repository,
        reservation_db_id,
    )
    .await?;

    let file_manifest = parse_file_manifest(&reservation.file_manifest)?;
    let file = first_manifest_file(&file_manifest)?;
    let storage_dir = app_state.storage_root().join(room_id.to_string());
    fs::create_dir_all(&storage_dir)
        .await
        .map_err(|e| AppError::internal(format!("创建存储目录失败：{}", e)))?;

    let final_storage_path = unique_storage_path(&storage_dir, &file.name)?;
    fs::rename(&final_file_path, &final_storage_path)
        .await
        .map_err(|e| AppError::internal(format!("移动文件失败：{}", e)))?;

    reservation_repository
        .consume_reservation(
            reservation_db_id,
            room_id,
            &verified.claims.jti,
            file.size,
            &reservation.file_manifest,
        )
        .await
        .map_err(|e| AppError::internal(format!("消费预留记录失败：{}", e)))?;
    reservation_repository
        .update_upload_status(reservation_db_id, UploadStatus::Completed)
        .await
        .map_err(|e| AppError::internal(format!("更新上传状态失败：{}", e)))?;

    cleanup_temp_dir(&temp_dir).await;

    let content_repository = RoomContentRepository::new(app_state.db_pool.clone());
    let created_content =
        create_content_record(&content_repository, room_id, file, &final_storage_path).await?;

    Ok(Json(FileMergeResponse {
        reservation_id: payload.reservation_id.clone(),
        merged_files: vec![MergedFileInfo {
            file_name: file.name.clone(),
            file_size: file.size,
            file_hash: payload.final_hash.clone(),
            content_id: created_content.id,
        }],
        message: "文件合并完成".to_string(),
    }))
}

fn parse_reservation_id(raw: &str) -> Result<i64, AppError> {
    raw.parse()
        .map_err(|_| AppError::validation("reservation_id 必须是数字"))
}

async fn load_reservation(
    repository: &RoomUploadReservationRepository,
    reservation_id: i64,
) -> Result<RoomUploadReservation, AppError> {
    repository
        .find_by_reservation_id(reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("查询预留记录失败：{}", e)))?
        .ok_or_else(|| AppError::not_found("预留记录不存在"))
}

fn reservation_id_or_error(reservation: &RoomUploadReservation) -> Result<i64, AppError> {
    reservation
        .id
        .ok_or_else(|| AppError::internal("预留 ID 不能为空"))
}

fn validate_reservation_for_merge(
    reservation: &RoomUploadReservation,
    room_id: i64,
) -> Result<(), AppError> {
    if reservation.room_id != room_id {
        return Err(AppError::permission_denied("预留记录不属于指定房间"));
    }
    if reservation.chunked_upload != Some(true) {
        return Err(AppError::validation("非分块上传预留记录"));
    }
    if reservation.expires_at < Utc::now().naive_utc() {
        return Err(AppError::permission_denied("预留记录已过期"));
    }
    if reservation.upload_status == Some(UploadStatus::Completed) {
        return Err(AppError::conflict("文件已完成合并"));
    }
    if reservation.upload_status == Some(UploadStatus::Failed) {
        return Err(AppError::conflict("上传已失败，无法合并"));
    }
    Ok(())
}

async fn load_uploaded_chunks(
    repository: &RoomChunkUploadRepository,
    reservation_id: i64,
    total_chunks: Option<i64>,
) -> Result<Vec<RoomChunkUpload>, AppError> {
    let chunks = repository
        .find_by_reservation_id(reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("查询分块记录失败：{}", e)))?;
    let total_chunks = total_chunks.unwrap_or(0);
    validate_uploaded_chunks(&chunks, total_chunks)?;

    let mut sorted_chunks = chunks;
    sorted_chunks.sort_by_key(|chunk| chunk.chunk_index);
    Ok(sorted_chunks)
}

fn validate_uploaded_chunks(chunks: &[RoomChunkUpload], total_chunks: i64) -> Result<(), AppError> {
    if total_chunks == 0 {
        return Err(AppError::validation("总分块数为 0"));
    }
    if chunks.len() != total_chunks as usize {
        return Err(AppError::validation(format!(
            "分块未全部上传，已上传：{}，总分块：{}",
            chunks.len(),
            total_chunks
        )));
    }
    for chunk in chunks {
        if !chunk.is_uploaded() {
            return Err(AppError::validation(format!(
                "分块{}未上传完成",
                chunk.chunk_index
            )));
        }
    }
    Ok(())
}

async fn merge_and_verify_chunks(
    chunks: &[RoomChunkUpload],
    final_file_path: &str,
    final_hash: &str,
    repository: &RoomUploadReservationRepository,
    reservation_id: i64,
) -> Result<(), AppError> {
    if let Err(e) = merge_chunks(chunks, final_file_path).await {
        mark_upload_failed(repository, reservation_id).await;
        return Err(AppError::internal(format!("文件合并失败：{}", e)));
    }

    match verify_file_hash(final_file_path, final_hash).await {
        Ok(true) => Ok(()),
        Ok(false) => {
            cleanup_failed_merge(final_file_path, repository, reservation_id).await;
            Err(AppError::validation("文件哈希验证失败"))
        }
        Err(e) => {
            cleanup_failed_merge(final_file_path, repository, reservation_id).await;
            Err(AppError::internal(format!("文件哈希验证失败：{}", e)))
        }
    }
}

async fn mark_upload_failed(repository: &RoomUploadReservationRepository, reservation_id: i64) {
    let _ = repository
        .update_upload_status(reservation_id, UploadStatus::Failed)
        .await;
}

async fn cleanup_failed_merge(
    final_file_path: &str,
    repository: &RoomUploadReservationRepository,
    reservation_id: i64,
) {
    let _ = fs::remove_file(final_file_path).await;
    mark_upload_failed(repository, reservation_id).await;
}

fn parse_file_manifest(manifest: &str) -> Result<Vec<UploadFileDescriptor>, AppError> {
    serde_json::from_str(manifest)
        .map_err(|e| AppError::internal(format!("解析文件清单失败：{}", e)))
}

fn first_manifest_file(
    file_manifest: &[UploadFileDescriptor],
) -> Result<&UploadFileDescriptor, AppError> {
    file_manifest
        .first()
        .ok_or_else(|| AppError::internal("文件清单为空"))
}

fn unique_storage_path(storage_dir: &StdPath, file_name: &str) -> Result<String, AppError> {
    let safe_file_name = sanitize_filename::sanitize(file_name);
    let mut final_filename = safe_file_name.clone();
    let mut counter = 1;
    let mut final_storage_path = storage_dir.join(&final_filename);

    while final_storage_path.exists() {
        let path = StdPath::new(&safe_file_name);
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
        final_storage_path = storage_dir.join(&final_filename);
        counter += 1;

        if counter > 1000 {
            return Err(AppError::internal("Too many files with the same name"));
        }
    }

    Ok(final_storage_path.to_string_lossy().into_owned())
}

async fn create_content_record(
    repository: &RoomContentRepository,
    room_id: i64,
    file: &UploadFileDescriptor,
    final_storage_path: &str,
) -> Result<RoomContent, AppError> {
    repository
        .create(&build_room_content(room_id, file, final_storage_path))
        .await
        .map_err(|e| AppError::internal(format!("创建内容记录失败：{}", e)))
}

fn build_room_content(
    room_id: i64,
    file: &UploadFileDescriptor,
    final_storage_path: &str,
) -> RoomContent {
    let now = chrono::Utc::now().naive_utc();
    RoomContent {
        id: None,
        room_id,
        content_type: ContentType::File,
        text: None,
        url: Some(file.name.clone()),
        path: Some(final_storage_path.to_string()),
        file_name: Some(file.name.clone()),
        size: Some(file.size),
        mime_type: Some(
            file.mime
                .clone()
                .unwrap_or_else(|| "application/octet-stream".to_string()),
        ),
        sequence_number: 0,
        created_at: now,
        updated_at: now,
    }
}

async fn cleanup_temp_dir(temp_dir: &str) {
    if let Err(e) = fs::remove_dir_all(temp_dir).await {
        logrs::error!("清理临时文件失败：{}", e);
    }
}

async fn merge_chunks(
    chunks: &[RoomChunkUpload],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut output_file = fs::File::create(output_path).await?;

    for chunk in chunks {
        let chunk_path = format!(
            "/tmp/elizabeth/chunks/{}/chunk_{}",
            chunk.reservation_id, chunk.chunk_index
        );
        let mut chunk_file = fs::File::open(&chunk_path).await?;

        let mut buffer = vec![0u8; chunk.chunk_size as usize];
        chunk_file.read_exact(&mut buffer).await?;

        output_file.write_all(&buffer).await?;
    }

    output_file.flush().await?;
    Ok(())
}

async fn verify_file_hash(
    file_path: &str,
    expected_hash: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let mut file = fs::File::open(file_path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let calculated_hash = hex::encode(hasher.finalize());
    Ok(calculated_hash == expected_hash)
}
