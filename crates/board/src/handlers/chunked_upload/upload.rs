use std::sync::Arc;

use axum::{
    Json,
    extract::{Multipart, Path, State},
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    dto::chunked_upload::{ChunkUploadRequest, ChunkUploadResponse},
    errors::{AppError, AppResult},
    models::room::{
        chunk_upload::{ChunkStatus, RoomChunkUpload},
        upload_reservation::UploadStatus,
    },
    repository::{
        room_chunk_upload_repository::{IRoomChunkUploadRepository, RoomChunkUploadRepository},
        room_repository::{IRoomRepository, RoomRepository},
        room_upload_reservation_repository::{
            IRoomUploadReservationRepository, RoomUploadReservationRepository,
        },
    },
    state::AppState,
    validation::RoomNameValidator,
};

type HandlerResult<T> = AppResult<Json<T>>;

/// 上传单个分块
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/uploads/chunks",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    request_body(content = ChunkUploadRequest, description = "分块上传请求", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "分块上传成功", body = ChunkUploadResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "房间不存在或预留记录不存在"),
        (status = 409, description = "分块已存在或冲突"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "chunked-upload"
)]
pub async fn upload_chunk(
    Path(room_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> HandlerResult<ChunkUploadResponse> {
    RoomNameValidator::validate_identifier(&room_name)?;

    let mut upload_token: Option<String> = None;
    let mut chunk_index: Option<i32> = None;
    let mut chunk_size: Option<i32> = None;
    let mut chunk_hash: Option<String> = None;
    let mut chunk_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(format!("解析 multipart 数据失败：{}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "upload_token" => {
                let token = field
                    .text()
                    .await
                    .map_err(|e| AppError::validation(format!("读取上传令牌失败：{}", e)))?;
                upload_token = Some(token);
            }
            "chunk_index" => {
                let index_str = field
                    .text()
                    .await
                    .map_err(|e| AppError::validation(format!("读取分块索引失败：{}", e)))?;
                chunk_index = Some(
                    index_str
                        .parse::<i32>()
                        .map_err(|_| AppError::validation("分块索引格式错误"))?,
                );
            }
            "chunk_size" => {
                let size_str = field
                    .text()
                    .await
                    .map_err(|e| AppError::validation(format!("读取分块大小失败：{}", e)))?;
                chunk_size = Some(
                    size_str
                        .parse::<i32>()
                        .map_err(|_| AppError::validation("分块大小格式错误"))?,
                );
            }
            "chunk_hash" => {
                let hash = field
                    .text()
                    .await
                    .map_err(|e| AppError::validation(format!("读取分块哈希失败：{}", e)))?;
                chunk_hash = Some(hash);
            }
            "chunk_data" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::validation(format!("读取分块数据失败：{}", e)))?;
                chunk_data = Some(data.to_vec());
            }
            _ => {}
        }
    }

    let upload_token = upload_token.ok_or_else(|| AppError::validation("缺少上传令牌"))?;
    let chunk_index = chunk_index.ok_or_else(|| AppError::validation("缺少分块索引"))?;
    let chunk_size = chunk_size.ok_or_else(|| AppError::validation("缺少分块大小"))?;
    let chunk_data = chunk_data.ok_or_else(|| AppError::validation("缺少分块数据"))?;

    if chunk_data.len() != chunk_size as usize {
        return Err(AppError::validation("分块数据大小不匹配"));
    }

    let reservation_repository = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation = reservation_repository
        .find_by_token(&upload_token)
        .await
        .map_err(|e| AppError::internal(format!("查询预留记录失败：{}", e)))?;
    let reservation = reservation.ok_or_else(|| AppError::not_found("预留记录不存在"))?;
    let reservation_id = reservation.id.expect("预留 ID 不能为空");

    if reservation.expires_at < Utc::now().naive_utc() {
        return Err(AppError::permission_denied("预留记录已过期"));
    }

    if reservation.chunked_upload != Some(true) {
        return Err(AppError::validation("非分块上传预留记录"));
    }

    let room_repository = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repository
        .find_by_id(reservation.room_id)
        .await
        .map_err(|e| AppError::internal(format!("查询房间失败：{}", e)))?;
    let room = room.ok_or_else(|| AppError::not_found("房间不存在"))?;

    if room.name != room_name {
        return Err(AppError::permission_denied("房间名称不匹配"));
    }

    let chunk_repository = RoomChunkUploadRepository::new(app_state.db_pool.clone());
    let existing_chunk = chunk_repository
        .find_by_reservation_and_index(reservation_id, chunk_index.into())
        .await
        .map_err(|e| AppError::internal(format!("查询分块记录失败：{}", e)))?;

    if existing_chunk.is_some() {
        return Err(AppError::conflict("分块已存在"));
    }

    let calculated_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&chunk_data);
        hex::encode(hasher.finalize())
    };

    if let Some(ref provided_hash) = chunk_hash
        && provided_hash != &calculated_hash
    {
        return Err(AppError::validation("分块哈希验证失败"));
    }

    let temp_dir = format!("/tmp/elizabeth/chunks/{reservation_id}/");
    fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| AppError::internal(format!("创建临时目录失败：{}", e)))?;

    let chunk_file_path = format!("{}chunk_{}", temp_dir, chunk_index);
    let mut file = fs::File::create(&chunk_file_path)
        .await
        .map_err(|e| AppError::internal(format!("创建分块文件失败：{}", e)))?;

    file.write_all(&chunk_data)
        .await
        .map_err(|e| AppError::internal(format!("写入分块数据失败：{}", e)))?;

    let chunk_record = RoomChunkUpload {
        id: None,
        reservation_id,
        chunk_index: chunk_index.into(),
        chunk_size: chunk_size.into(),
        chunk_hash: Some(calculated_hash.clone()),
        upload_status: ChunkStatus::Uploaded,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    chunk_repository
        .create(&chunk_record)
        .await
        .map_err(|e| AppError::internal(format!("创建分块记录失败：{}", e)))?;

    let uploaded_chunks = chunk_repository
        .count_by_reservation_id(reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("统计已上传分块数失败：{}", e)))?;

    reservation_repository
        .update_uploaded_chunks(reservation_id, uploaded_chunks as i64)
        .await
        .map_err(|e| AppError::internal(format!("更新上传进度失败：{}", e)))?;

    let response = ChunkUploadResponse {
        chunk_index,
        chunk_size: chunk_size.into(),
        chunk_hash: Some(calculated_hash),
        upload_status: ChunkStatus::Uploaded,
        uploaded_at: Utc::now().naive_utc(),
    };

    Ok(Json(response))
}
