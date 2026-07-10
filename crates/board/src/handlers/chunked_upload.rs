use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::chunked_upload::{
        ChunkStatusInfo, ChunkedUploadPreparationRequest, ChunkedUploadPreparationResponse,
        ReservedFileInfo, UploadStatusQuery, UploadStatusResponse,
    },
    errors::{AppError, AppResult},
    models::room::upload_reservation::UploadStatus,
    repository::room_chunk_upload_repository::{
        IRoomChunkUploadRepository, RoomChunkUploadRepository,
    },
    repository::room_upload_reservation_repository::{
        IRoomUploadReservationRepository, RoomUploadReservationRepository,
    },
    state::AppState,
    validation::RoomNameValidator,
};

use super::{AuthToken, VerifiedRoomToken, verify_room_token};
type HandlerResult<T> = AppResult<Json<T>>;

pub mod complete;
pub mod upload;

/// 预留分块上传空间
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/uploads/chunks/prepare",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    request_body = ChunkedUploadPreparationRequest,
    responses(
        (status = 200, description = "预留成功", body = ChunkedUploadPreparationResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "房间不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "chunked-upload"
)]
pub async fn prepare_chunked_upload(
    Path(room_name): Path<String>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<ChunkedUploadPreparationRequest>,
) -> HandlerResult<ChunkedUploadPreparationResponse> {
    // 验证房间名称
    RoomNameValidator::validate_identifier(&room_name)?;

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

    // 验证 token 并获取房间
    let verified = verify_room_token(app_state.clone(), &room_name, &token).await?;
    let room = verified.room;

    // 检查 token 是否有编辑权限
    let token_permission = verified.claims.as_permission();
    if !token_permission.can_edit() {
        return Err(AppError::permission_denied("token 无编辑权限"));
    }

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
            &verified.claims.jti,
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
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<UploadStatusQuery>,
) -> HandlerResult<UploadStatusResponse> {
    // 验证房间名称
    RoomNameValidator::validate_identifier(&room_name)?;

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

    let verified = verify_room_token(app_state.clone(), &room_name, &token).await?;

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
    ensure_reservation_access(&reservation, &verified)?;

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

/// 取消分块上传，清理临时文件并释放预留空间
#[utoipa::path(
    delete,
    path = "/api/v1/rooms/{name}/uploads/chunks/{reservation_id}",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("reservation_id" = i64, Path, description = "预留记录 ID")
    ),
    responses(
        (status = 200, description = "上传已取消"),
        (status = 404, description = "房间或预留记录不存在"),
        (status = 409, description = "上传已完成，无法取消")
    ),
    tag = "chunked-upload"
)]
pub async fn cancel_chunked_upload(
    Path((room_name, reservation_id)): Path<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<serde_json::Value> {
    RoomNameValidator::validate_identifier(&room_name)?;

    let verified = verify_room_token(app_state.clone(), &room_name, &token).await?;

    let reservation_repository = RoomUploadReservationRepository::new(app_state.db_pool.clone());
    let reservation = reservation_repository
        .find_by_reservation_id(reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("查询预留记录失败：{}", e)))?;
    let reservation = reservation.ok_or_else(|| AppError::not_found("预留记录不存在"))?;

    ensure_reservation_access(&reservation, &verified)?;

    if reservation.consumed_at.is_some() {
        return Err(AppError::conflict("上传已完成，无法取消"));
    }

    // 清理临时分块文件
    if let Err(e) = crate::chunk_temp_storage::remove_reservation_dir(reservation_id).await {
        logrs::error!("清理临时分块文件失败：{}", e);
    }

    // 释放预留空间
    reservation_repository
        .release_if_pending(reservation_id)
        .await
        .map_err(|e| AppError::internal(format!("释放预留空间失败：{}", e)))?;

    Ok(Json(serde_json::json!({
        "message": "上传已取消",
        "reservation_id": reservation_id,
    })))
}

pub(crate) fn ensure_reservation_access(
    reservation: &crate::models::RoomUploadReservation,
    verified: &VerifiedRoomToken,
) -> Result<(), AppError> {
    if reservation.room_id != verified.claims.room_id {
        return Err(AppError::permission_denied("预留记录不属于指定房间"));
    }
    if reservation.owner_token_jti != verified.claims.jti {
        return Err(AppError::permission_denied("预留记录不属于当前会话"));
    }
    if !verified.room.permission.can_edit() || !verified.claims.as_permission().can_edit() {
        return Err(AppError::permission_denied("房间或会话无编辑权限"));
    }
    if reservation.expires_at <= Utc::now().naive_utc() {
        return Err(AppError::permission_denied("预留记录已过期"));
    }
    Ok(())
}
