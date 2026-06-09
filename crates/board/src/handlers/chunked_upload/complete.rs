use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
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
        upload_reservation::{UploadFileDescriptor, UploadStatus},
    },
    repository::{
        IRoomContentRepository, IRoomRepository,
        room_chunk_upload_repository::{IRoomChunkUploadRepository, RoomChunkUploadRepository},
        room_content_repository::RoomContentRepository,
        room_repository::RoomRepository,
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
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<FileMergeRequest>,
) -> HandlerResult<FileMergeResponse> {
    RoomNameValidator::validate_identifier(&room_name)?;

    let room_repository = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repository
        .find_by_name(&room_name)
        .await
        .map_err(|e| AppError::internal(format!("数据库查询错误：{}", e)))?;
    let room = room.ok_or_else(|| AppError::not_found("房间不存在"))?;

    let reservation_repository = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation_id: i64 = payload
        .reservation_id
        .parse()
        .map_err(|_| AppError::validation("reservation_id 必须是数字"))?;
    let reservation = reservation_repository
        .find_by_reservation_id(reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("查询预留记录失败：{}", e)))?;
    let reservation = reservation.ok_or_else(|| AppError::not_found("预留记录不存在"))?;

    if reservation.room_id != room.id.expect("房间 ID 不能为空") {
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

    let chunk_repository = RoomChunkUploadRepository::new(app_state.db_pool.clone());
    let reservation_db_id = reservation.id.expect("预留 ID 不能为空");
    let chunks = chunk_repository
        .find_by_reservation_id(reservation_db_id)
        .await
        .map_err(|e| AppError::internal(format!("查询分块记录失败：{}", e)))?;

    let total_chunks = reservation.total_chunks.unwrap_or(0);
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

    for chunk in &chunks {
        if !chunk.is_uploaded() {
            return Err(AppError::validation(format!(
                "分块{}未上传完成",
                chunk.chunk_index
            )));
        }
    }

    let mut sorted_chunks = chunks;
    sorted_chunks.sort_by_key(|chunk| chunk.chunk_index);

    let temp_dir = format!("/tmp/elizabeth/chunks/{}/", payload.reservation_id);
    let final_file_path = format!("{}/merged_file", temp_dir);

    match merge_chunks(&sorted_chunks, &final_file_path).await {
        Ok(_) => match verify_file_hash(&final_file_path, &payload.final_hash).await {
            Ok(true) => {
                let storage_dir = format!("storage/rooms/{}/", room.id.expect("房间 ID 不能为空"));
                fs::create_dir_all(&storage_dir)
                    .await
                    .map_err(|e| AppError::internal(format!("创建存储目录失败：{}", e)))?;

                let file_manifest: Vec<UploadFileDescriptor> =
                    serde_json::from_str(&reservation.file_manifest)
                        .map_err(|e| AppError::internal(format!("解析文件清单失败：{}", e)))?;

                if file_manifest.is_empty() {
                    return Err(AppError::internal("文件清单为空"));
                }

                let file_name = &file_manifest[0].name;
                let safe_file_name = sanitize_filename::sanitize(file_name);
                let mut final_filename = safe_file_name.clone();
                let mut counter = 1;
                let mut final_storage_path = format!("{}{}", storage_dir, final_filename);

                while std::path::Path::new(&final_storage_path).exists() {
                    let path = std::path::Path::new(&safe_file_name);
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

                    final_storage_path = format!("{}{}", storage_dir, final_filename);
                    counter += 1;

                    if counter > 1000 {
                        return Err(AppError::internal("Too many files with the same name"));
                    }
                }

                fs::rename(&final_file_path, &final_storage_path)
                    .await
                    .map_err(|e| AppError::internal(format!("移动文件失败：{}", e)))?;

                reservation_repository
                    .update_upload_status(reservation_db_id, UploadStatus::Completed)
                    .await
                    .map_err(|e| AppError::internal(format!("更新上传状态失败：{}", e)))?;

                reservation_repository
                    .consume_upload(reservation_db_id)
                    .await
                    .map_err(|e| AppError::internal(format!("消费预留记录失败：{}", e)))?;

                if let Err(e) = fs::remove_dir_all(&temp_dir).await {
                    logrs::error!("清理临时文件失败：{}", e);
                }

                let file_name = &file_manifest[0].name;
                let mime_type = file_manifest[0]
                    .mime
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string());

                let content_repository = RoomContentRepository::new(app_state.db_pool.clone());
                let now = chrono::Utc::now().naive_utc();
                let room_id_value: i64 = room.id.expect("房间 ID 不能为空");
                let room_content = RoomContent {
                    id: None,
                    room_id: room_id_value,
                    content_type: ContentType::File,
                    text: None,
                    url: Some(file_name.clone()),
                    path: Some(final_storage_path.clone()),
                    file_name: Some(file_name.clone()),
                    size: Some(file_manifest[0].size),
                    mime_type: Some(mime_type),
                    sequence_number: 0,
                    created_at: now,
                    updated_at: now,
                };

                let created_content = content_repository
                    .create(&room_content)
                    .await
                    .map_err(|e| AppError::internal(format!("创建内容记录失败：{}", e)))?;

                let file_size = file_manifest[0].size;
                let reserved_size = reservation.reserved_size;
                let mut updated_room = room.clone();
                updated_room.current_size =
                    (updated_room.current_size - reserved_size + file_size).max(0);
                let room_repository = RoomRepository::new(app_state.db_pool.clone());
                room_repository
                    .update(&updated_room)
                    .await
                    .map_err(|e| AppError::internal(format!("更新房间大小失败：{}", e)))?;

                let merged_file_info = MergedFileInfo {
                    file_name: file_name.clone(),
                    file_size: file_manifest[0].size,
                    file_hash: payload.final_hash.clone(),
                    content_id: created_content.id,
                };

                let response = FileMergeResponse {
                    reservation_id: payload.reservation_id.clone(),
                    merged_files: vec![merged_file_info],
                    message: "文件合并完成".to_string(),
                };

                Ok(Json(response))
            }
            Ok(false) => {
                let _ = fs::remove_file(&final_file_path).await;
                let _ = reservation_repository
                    .update_upload_status(reservation_db_id, UploadStatus::Failed)
                    .await;

                Err(AppError::validation("文件哈希验证失败"))
            }
            Err(e) => {
                let _ = fs::remove_file(&final_file_path).await;
                let _ = reservation_repository
                    .update_upload_status(reservation_db_id, UploadStatus::Failed)
                    .await;

                Err(AppError::internal(format!("文件哈希验证失败：{}", e)))
            }
        },
        Err(e) => {
            let _ = reservation_repository
                .update_upload_status(reservation_db_id, UploadStatus::Failed)
                .await;

            Err(AppError::internal(format!("文件合并失败：{}", e)))
        }
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
