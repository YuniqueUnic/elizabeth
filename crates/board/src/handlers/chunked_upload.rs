use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use chrono::{NaiveDateTime, Utc};
use hex;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::{
    dto::chunked_upload::{
        ChunkStatusInfo, ChunkUploadRequest, ChunkUploadResponse, ChunkedUploadPreparationRequest,
        ChunkedUploadPreparationResponse, FileMergeRequest, FileMergeResponse, MergedFileInfo,
        ReservedFileInfo, UploadStatusQuery, UploadStatusResponse,
    },
    errors::{AppError, AppResult},
    models::room::{
        chunk_upload::{ChunkStatus, RoomChunkUpload},
        upload_reservation::{UploadFileDescriptor, UploadStatus},
    },
    repository::room_chunk_upload_repository::{
        IRoomChunkUploadRepository, RoomChunkUploadRepository,
    },
    repository::room_repository::{IRoomRepository, RoomRepository},
    repository::room_upload_reservation_repository::{
        IRoomUploadReservationRepository, RoomUploadReservationRepository,
    },
    state::AppState,
};
type HandlerResult<T> = AppResult<Json<T>>;

/// 预留分块上传空间
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/uploads/chunks/prepare",
    params(
        ("name" = String, Path, description = "房间名称")
    ),
    request_body = ChunkedUploadPreparationRequest,
    responses(
        (status = 200, description = "预留成功", body = ChunkedUploadPreparationResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "房间不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "chunked-upload"
)]
pub async fn prepare_chunked_upload(
    Path(room_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<ChunkedUploadPreparationRequest>,
) -> HandlerResult<ChunkedUploadPreparationResponse> {
    // 验证房间名称
    if room_name.is_empty() {
        return Err(AppError::validation("房间名称不能为空"));
    }

    // 验证请求参数
    if payload.files.is_empty() {
        return Err(AppError::validation("文件列表不能为空"));
    }

    // 验证每个文件的信息
    for file in &payload.files {
        if file.name.is_empty() {
            return Err(AppError::validation("文件名不能为空"));
        }
        if file.size <= 0 {
            return Err(AppError::validation("文件大小必须大于 0"));
        }
        if let Some(chunk_size) = file.chunk_size
            && chunk_size <= 0
        {
            return Err(AppError::validation("分块大小必须大于 0"));
        }
    }

    // 查找房间
    let room_repository = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repository
        .find_by_name(&room_name)
        .await
        .map_err(|e| AppError::internal(format!("数据库查询错误：{}", e)))?;

    let room = room.ok_or_else(|| AppError::not_found("房间不存在"))?;

    // 计算总预留大小
    let total_reserved_size: i64 = payload.files.iter().map(|f| f.size).sum();

    if !room.can_add_content(total_reserved_size) {
        return Err(AppError::permission_denied("房间空间不足或无上传权限"));
    }

    let upload_token = Uuid::new_v4().to_string();
    let expires_at = Utc::now().naive_utc() + app_state.upload_reservation_ttl();
    let file_manifest = serde_json::to_string(&payload.files)
        .map_err(|e| AppError::internal(format!("序列化文件清单失败：{}", e)))?;

    let reservation_repository = RoomUploadReservationRepository::new(app_state.db_pool.clone());

    // 计算每个文件的分块信息并构建响应
    let mut reserved_files = Vec::new();
    let mut total_chunks = 0;

    for file in payload.files {
        let chunk_size = file.chunk_size.unwrap_or(1024 * 1024) as i64; // 默认 1MB
        let file_total_chunks = (file.size + chunk_size - 1) / chunk_size; // 向上取整
        total_chunks += file_total_chunks;

        reserved_files.push(ReservedFileInfo {
            name: file.name,
            size: file.size,
            mime: file.mime,
            chunk_size,
            total_chunks: file_total_chunks,
            file_hash: file.file_hash,
        });
    }

    // 使用现有的 reserve_upload 方法创建预留记录
    let (saved_reservation, _) = reservation_repository
        .reserve_upload(
            &room,
            &upload_token,
            &file_manifest,
            total_reserved_size,
            app_state.upload_reservation_ttl(),
        )
        .await
        .map_err(|e| AppError::internal(format!("创建预留记录失败：{}", e)))?;

    // 设置 chunked_upload 标记和 chunk 信息
    let db_reservation_id = saved_reservation.id.expect("预留 ID 不能为空");
    let status_str = UploadStatus::Pending.to_string();
    let reservation_chunk_size = reserved_files
        .first()
        .map(|file| file.chunk_size)
        .unwrap_or(1024 * 1024);

    sqlx::query(
        r#"
        UPDATE room_upload_reservations
        SET chunked_upload = true,
            total_chunks = $1,
            uploaded_chunks = 0,
            chunk_size = $2,
            upload_status = $3
        WHERE id = $4
        "#,
    )
    .bind(total_chunks)
    .bind(reservation_chunk_size)
    .bind(&status_str)
    .bind(db_reservation_id)
    .execute(app_state.db_pool.as_ref())
    .await
    .map_err(|e| AppError::internal(format!("更新预留记录失败：{}", e)))?;

    // 构建响应
    let response = ChunkedUploadPreparationResponse {
        reservation_id: db_reservation_id.to_string(),
        upload_token,
        expires_at,
        files: reserved_files,
    };

    Ok(Json(response))
}

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
    // 验证房间名称
    if room_name.is_empty() {
        return Err(AppError::validation("房间名称不能为空"));
    }

    // 解析 multipart 数据
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
            _ => {
                // 忽略未知字段
            }
        }
    }

    // 验证必需字段
    let upload_token = upload_token.ok_or_else(|| AppError::validation("缺少上传令牌"))?;

    let chunk_index = chunk_index.ok_or_else(|| AppError::validation("缺少分块索引"))?;

    let chunk_size = chunk_size.ok_or_else(|| AppError::validation("缺少分块大小"))?;

    let chunk_data = chunk_data.ok_or_else(|| AppError::validation("缺少分块数据"))?;

    // 验证分块数据大小
    if chunk_data.len() != chunk_size as usize {
        return Err(AppError::validation("分块数据大小不匹配"));
    }

    // 查找预留记录
    let reservation_repository = RoomUploadReservationRepository::new(app_state.db_pool.clone());

    let reservation = reservation_repository
        .find_by_token(&upload_token)
        .await
        .map_err(|e| AppError::internal(format!("查询预留记录失败：{}", e)))?;

    let reservation = reservation.ok_or_else(|| AppError::not_found("预留记录不存在"))?;
    let reservation_id = reservation.id.expect("预留 ID 不能为空");

    // 验证预留记录是否有效
    if reservation.expires_at < Utc::now().naive_utc() {
        return Err(AppError::permission_denied("预留记录已过期"));
    }

    // 验证是否为分块上传
    if reservation.chunked_upload != Some(true) {
        return Err(AppError::validation("非分块上传预留记录"));
    }

    // 验证房间名称
    let room_repository = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repository
        .find_by_id(reservation.room_id)
        .await
        .map_err(|e| AppError::internal(format!("查询房间失败：{}", e)))?;

    let room = room.ok_or_else(|| AppError::not_found("房间不存在"))?;

    if room.name != room_name {
        return Err(AppError::permission_denied("房间名称不匹配"));
    }

    // 检查分块是否已存在
    let chunk_repository = RoomChunkUploadRepository::new(app_state.db_pool.clone());
    let existing_chunk = chunk_repository
        .find_by_reservation_and_index(reservation_id, chunk_index.into())
        .await
        .map_err(|e| AppError::internal(format!("查询分块记录失败：{}", e)))?;

    if existing_chunk.is_some() {
        return Err(AppError::conflict("分块已存在"));
    }

    // 计算分块哈希
    let calculated_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&chunk_data);
        hex::encode(hasher.finalize())
    };

    // 验证分块哈希（如果提供）
    if let Some(ref provided_hash) = chunk_hash
        && provided_hash != &calculated_hash
    {
        return Err(AppError::validation("分块哈希验证失败"));
    }

    // 创建临时存储目录
    let temp_dir = format!("/tmp/elizabeth/chunks/{reservation_id}/");
    fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| AppError::internal(format!("创建临时目录失败：{}", e)))?;

    // 保存分块数据到临时文件
    let chunk_file_path = format!("{}chunk_{}", temp_dir, chunk_index);
    let mut file = fs::File::create(&chunk_file_path)
        .await
        .map_err(|e| AppError::internal(format!("创建分块文件失败：{}", e)))?;

    use tokio::io::AsyncWriteExt;
    file.write_all(&chunk_data)
        .await
        .map_err(|e| AppError::internal(format!("写入分块数据失败：{}", e)))?;

    // 创建分块记录
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

    // 保存分块记录到数据库
    chunk_repository
        .create(&chunk_record)
        .await
        .map_err(|e| AppError::internal(format!("创建分块记录失败：{}", e)))?;

    // 更新预留记录的上传进度
    let uploaded_chunks = chunk_repository
        .count_by_reservation_id(reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("统计已上传分块数失败：{}", e)))?;

    // 更新预留记录
    reservation_repository
        .update_uploaded_chunks(reservation_id, uploaded_chunks as i64)
        .await
        .map_err(|e| AppError::internal(format!("更新上传进度失败：{}", e)))?;

    // 构建响应
    let response = ChunkUploadResponse {
        chunk_index,
        chunk_size: chunk_size.into(),
        chunk_hash: Some(calculated_hash),
        upload_status: ChunkStatus::Uploaded,
        uploaded_at: Utc::now().naive_utc(),
    };

    Ok(Json(response))
}

/// 查询上传状态
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}/uploads/chunks/status",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = Option<String>, Query, description = "上传令牌"),
        ("reservation_id" = Option<String>, Query, description = "预留 ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = UploadStatusResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "房间不存在或预留记录不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "chunked-upload"
)]
pub async fn get_upload_status(
    Path(room_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<UploadStatusQuery>,
) -> HandlerResult<UploadStatusResponse> {
    // 验证房间名称
    if room_name.is_empty() {
        return Err(AppError::validation("房间名称不能为空"));
    }

    // 验证查询参数
    if query.upload_token.is_none() && query.reservation_id.is_none() {
        return Err(AppError::validation(
            "必须提供 upload_token 或 reservation_id",
        ));
    }

    if query.upload_token.is_some() && query.reservation_id.is_some() {
        return Err(AppError::validation(
            "upload_token 和 reservation_id 只能提供一个",
        ));
    }

    // 查找房间
    let room_repository = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repository
        .find_by_name(&room_name)
        .await
        .map_err(|e| AppError::internal(format!("数据库查询错误：{}", e)))?;

    let room = room.ok_or_else(|| AppError::not_found("房间不存在"))?;

    // 查找预留记录
    let reservation_repository = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation = if let Some(ref token) = query.upload_token {
        reservation_repository
            .find_by_token(token)
            .await
            .map_err(|e| AppError::internal(format!("查询预留记录失败：{}", e)))?
    } else if let Some(ref reservation_id) = query.reservation_id {
        let reservation_id: i64 = reservation_id
            .parse()
            .map_err(|_| AppError::validation("reservation_id 必须是数字"))?;
        reservation_repository
            .find_by_reservation_id(reservation_id)
            .await
            .map_err(|e| AppError::internal(format!("查询预留记录失败：{}", e)))?
    } else {
        return Err(AppError::validation("缺少查询参数"));
    };

    let reservation = reservation.ok_or_else(|| AppError::not_found("预留记录不存在"))?;

    // 验证预留记录是否属于指定房间
    if reservation.room_id != room.id.expect("房间 ID 不能为空") {
        return Err(AppError::permission_denied("预留记录不属于指定房间"));
    }

    // 验证预留记录是否为分块上传
    if reservation.chunked_upload != Some(true) {
        return Err(AppError::validation("非分块上传预留记录"));
    }

    // 检查是否过期
    let now = Utc::now().naive_utc();
    let is_expired = reservation.expires_at < now;
    let remaining_seconds = if is_expired {
        None
    } else {
        Some((reservation.expires_at - now).num_seconds())
    };

    // 查询分块记录
    let chunk_repository = RoomChunkUploadRepository::new(app_state.db_pool.clone());
    let reservation_db_id = reservation.id.expect("预留 ID 不能为空");
    let chunks = chunk_repository
        .find_by_reservation_id(reservation_db_id)
        .await
        .map_err(|e| AppError::internal(format!("查询分块记录失败：{}", e)))?;

    // 计算统计信息
    let total_chunks = reservation.total_chunks.unwrap_or(0);
    let uploaded_chunks = chunks.len() as i64;
    let progress_percentage = if total_chunks > 0 {
        (uploaded_chunks as f64 / total_chunks as f64) * 100.0
    } else {
        0.0
    };

    // 计算已上传大小
    let uploaded_size: i64 = chunks.iter().map(|chunk| chunk.chunk_size).sum();

    // 构建分块详细信息
    let chunk_details: Vec<ChunkStatusInfo> = chunks
        .into_iter()
        .map(|chunk| ChunkStatusInfo {
            chunk_index: chunk.chunk_index,
            chunk_size: chunk.chunk_size,
            chunk_hash: chunk.chunk_hash,
            upload_status: chunk.upload_status,
            uploaded_at: Some(chunk.updated_at),
        })
        .collect();

    // 确定上传状态
    let upload_status = if is_expired {
        UploadStatus::Expired
    } else if uploaded_chunks == 0 {
        UploadStatus::Pending
    } else if uploaded_chunks >= total_chunks && total_chunks > 0 {
        UploadStatus::Completed
    } else {
        UploadStatus::Uploading
    };

    // 构建响应
    let response = UploadStatusResponse {
        reservation_id: reservation.id.expect("预留 ID 不能为空").to_string(),
        upload_token: reservation.token_jti,
        upload_status,
        total_chunks,
        uploaded_chunks,
        progress_percentage,
        expires_at: reservation.expires_at,
        chunk_details,
        reserved_size: reservation.reserved_size,
        uploaded_size,
        is_expired,
        remaining_seconds,
    };

    Ok(Json(response))
}

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
    // 验证房间名称
    if room_name.is_empty() {
        return Err(AppError::validation("房间名称不能为空"));
    }

    // 查找房间
    let room_repository = RoomRepository::new(app_state.db_pool.clone());
    let room = room_repository
        .find_by_name(&room_name)
        .await
        .map_err(|e| AppError::internal(format!("数据库查询错误：{}", e)))?;

    let room = room.ok_or_else(|| AppError::not_found("房间不存在"))?;

    // 查找预留记录
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

    // 验证预留记录是否属于指定房间
    if reservation.room_id != room.id.expect("房间 ID 不能为空") {
        return Err(AppError::permission_denied("预留记录不属于指定房间"));
    }

    // 验证预留记录是否为分块上传
    if reservation.chunked_upload != Some(true) {
        return Err(AppError::validation("非分块上传预留记录"));
    }

    // 验证预留记录是否已过期
    if reservation.expires_at < Utc::now().naive_utc() {
        return Err(AppError::permission_denied("预留记录已过期"));
    }

    // 验证预留记录状态
    if reservation.upload_status == Some(UploadStatus::Completed) {
        return Err(AppError::conflict("文件已完成合并"));
    }

    if reservation.upload_status == Some(UploadStatus::Failed) {
        return Err(AppError::conflict("上传已失败，无法合并"));
    }

    // 查询所有分块记录
    let chunk_repository = RoomChunkUploadRepository::new(app_state.db_pool.clone());
    let reservation_db_id = reservation.id.expect("预留 ID 不能为空");
    let chunks = chunk_repository
        .find_by_reservation_id(reservation_db_id)
        .await
        .map_err(|e| AppError::internal(format!("查询分块记录失败：{}", e)))?;

    // 验证所有分块是否已上传
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

    // 验证所有分块状态
    for chunk in &chunks {
        if !chunk.is_uploaded() {
            return Err(AppError::validation(format!(
                "分块{}未上传完成",
                chunk.chunk_index
            )));
        }
    }

    // 按分块索引排序
    let mut sorted_chunks = chunks;
    sorted_chunks.sort_by_key(|chunk| chunk.chunk_index);

    // 创建临时目录和合并文件
    let temp_dir = format!("/tmp/elizabeth/chunks/{}/", payload.reservation_id);
    let final_file_path = format!("{}/merged_file", temp_dir);

    // 执行文件合并
    match merge_chunks(&sorted_chunks, &final_file_path).await {
        Ok(_) => {
            // 验证合并后文件的哈希
            match verify_file_hash(&final_file_path, &payload.final_hash).await {
                Ok(true) => {
                    // 移动文件到最终存储位置
                    let storage_dir =
                        format!("storage/rooms/{}/", room.id.expect("房间 ID 不能为空"));
                    fs::create_dir_all(&storage_dir)
                        .await
                        .map_err(|e| AppError::internal(format!("创建存储目录失败：{}", e)))?;

                    // 从文件清单中获取文件名
                    let file_manifest: Vec<UploadFileDescriptor> =
                        serde_json::from_str(&reservation.file_manifest)
                            .map_err(|e| AppError::internal(format!("解析文件清单失败：{}", e)))?;

                    if file_manifest.is_empty() {
                        return Err(AppError::internal("文件清单为空"));
                    }

                    let file_name = &file_manifest[0].name;

                    // ✅ FIX: Handle filename conflicts by adding suffix
                    let safe_file_name = sanitize_filename::sanitize(file_name);
                    let mut final_filename = safe_file_name.clone();
                    let mut counter = 1;
                    let mut final_storage_path = format!("{}{}", storage_dir, final_filename);

                    while std::path::Path::new(&final_storage_path).exists() {
                        // Extract base name and extension
                        let path = std::path::Path::new(&safe_file_name);
                        let stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(&safe_file_name);
                        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                        // Create new filename with counter
                        final_filename = if extension.is_empty() {
                            format!("{}({})", stem, counter)
                        } else {
                            format!("{}({}).{}", stem, counter, extension)
                        };

                        final_storage_path = format!("{}{}", storage_dir, final_filename);
                        counter += 1;

                        // Prevent infinite loop
                        if counter > 1000 {
                            return Err(AppError::internal("Too many files with the same name"));
                        }
                    }

                    // 移动合并后的文件到最终位置
                    fs::rename(&final_file_path, &final_storage_path)
                        .await
                        .map_err(|e| AppError::internal(format!("移动文件失败：{}", e)))?;

                    // 更新预留记录状态为已完成
                    reservation_repository
                        .update_upload_status(reservation_db_id, UploadStatus::Completed)
                        .await
                        .map_err(|e| AppError::internal(format!("更新上传状态失败：{}", e)))?;

                    // 设置预留记录为已消费
                    reservation_repository
                        .consume_upload(reservation_db_id)
                        .await
                        .map_err(|e| AppError::internal(format!("消费预留记录失败：{}", e)))?;

                    // 清理临时分块文件
                    if let Err(e) = fs::remove_dir_all(&temp_dir).await {
                        logrs::error!("清理临时文件失败：{}", e);
                    }

                    // 创建 RoomContent 记录
                    let file_name = &file_manifest[0].name;
                    let mime_type = file_manifest[0]
                        .mime
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_string());

                    use crate::repository::IRoomContentRepository;
                    let content_repository =
                        crate::repository::room_content_repository::RoomContentRepository::new(
                            app_state.db_pool.clone(),
                        );

                    // 构建 RoomContent 对象
                    let now = chrono::Utc::now().naive_utc();
                    let room_id_value: i64 = room.id.expect("房间 ID 不能为空");
                    let room_content = crate::models::room::content::RoomContent {
                        id: None,
                        room_id: room_id_value,
                        content_type: crate::models::room::content::ContentType::File,
                        text: None,
                        url: Some(file_name.clone()),
                        path: Some(final_storage_path.clone()),
                        file_name: Some(file_name.clone()), // 保存原始文件名
                        size: Some(file_manifest[0].size),
                        mime_type: Some(mime_type),
                        created_at: now,
                        updated_at: now,
                    };

                    let created_content = content_repository
                        .create(&room_content)
                        .await
                        .map_err(|e| AppError::internal(format!("创建内容记录失败：{}", e)))?;

                    // 更新房间的 current_size
                    let file_size = file_manifest[0].size;
                    let mut updated_room = room.clone();
                    updated_room.current_size += file_size;
                    use crate::repository::IRoomRepository;
                    let room_repository = RoomRepository::new(app_state.db_pool.clone());
                    room_repository
                        .update(&updated_room)
                        .await
                        .map_err(|e| AppError::internal(format!("更新房间大小失败：{}", e)))?;

                    // 构建响应
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
                    // 哈希验证失败
                    // 清理合并文件
                    let _ = fs::remove_file(&final_file_path).await;

                    // 更新预留记录状态为失败
                    let _ = reservation_repository
                        .update_upload_status(reservation_db_id, UploadStatus::Failed)
                        .await;

                    Err(AppError::validation("文件哈希验证失败"))
                }
                Err(e) => {
                    // 哈希验证错误
                    // 清理合并文件
                    let _ = fs::remove_file(&final_file_path).await;

                    // 更新预留记录状态为失败
                    let _ = reservation_repository
                        .update_upload_status(reservation_db_id, UploadStatus::Failed)
                        .await;

                    Err(AppError::internal(format!("文件哈希验证失败：{}", e)))
                }
            }
        }
        Err(e) => {
            // 文件合并失败
            // 更新预留记录状态为失败
            let _ = reservation_repository
                .update_upload_status(reservation_db_id, UploadStatus::Failed)
                .await;

            Err(AppError::internal(format!("文件合并失败：{}", e)))
        }
    }
}

/// 合并分块文件
async fn merge_chunks(
    chunks: &[RoomChunkUpload],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    let mut output_file = File::create(output_path).await?;

    for chunk in chunks {
        let chunk_path = format!(
            "/tmp/elizabeth/chunks/{}/chunk_{}",
            chunk.reservation_id, chunk.chunk_index
        );
        let mut chunk_file = File::open(&chunk_path).await?;

        let mut buffer = vec![0u8; chunk.chunk_size as usize];
        chunk_file.read_exact(&mut buffer).await?;

        output_file.write_all(&buffer).await?;
    }

    output_file.flush().await?;
    Ok(())
}

/// 验证文件哈希
async fn verify_file_hash(
    file_path: &str,
    expected_hash: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    let mut file = File::open(file_path).await?;
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
