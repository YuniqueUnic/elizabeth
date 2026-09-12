use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::HeaderMap;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::dto::{
    AdminConfigResponse, AdminRoomDetailResponse, AdminRoomListResponse, AdminRoomView,
    AdminStatsResponse, AdminStorageResponse, DeleteRoomResponse, FullRoomGcStatusView,
    RunRoomGcResponse,
};
use crate::errors::{AppError, AppResult};
use crate::repository::{AdminConsoleRepository, IRoomRepository, RoomRepository};
use crate::state::AppState;

type HandlerResult<T> = Result<Json<T>, AppError>;

const DEFAULT_ADMIN_LIMIT: u32 = 100;
const MAX_ADMIN_LIMIT: u32 = 1000;
const ADMIN_TOKEN_ENV: &str = "ELIZABETH_ADMIN_TOKEN";
const ADMIN_TOKEN_HEADER: &str = "X-Elizabeth-Admin-Token";

#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminLimitQuery {
    pub limit: Option<u32>,
}

pub(crate) fn validate_admin_credential(provided: Option<&str>) -> AppResult<()> {
    let expected = std::env::var(ADMIN_TOKEN_ENV).unwrap_or_default();
    if expected.trim().is_empty() {
        return Err(AppError::authorization(format!(
            "Admin API disabled (set {ADMIN_TOKEN_ENV})"
        )));
    }

    if provided.unwrap_or("") != expected {
        return Err(AppError::authorization("Invalid admin token"));
    }

    Ok(())
}

fn ensure_admin(headers: &HeaderMap) -> AppResult<()> {
    validate_admin_credential(
        headers
            .get(ADMIN_TOKEN_HEADER)
            .and_then(|value| value.to_str().ok()),
    )
}

fn clamp_limit(limit: Option<u32>) -> u32 {
    let limit = limit.unwrap_or(DEFAULT_ADMIN_LIMIT);
    limit.clamp(1, MAX_ADMIN_LIMIT)
}

/// 列出“无过期且已满”的房间（用于运维/排查 GC 状态）
#[utoipa::path(
    get,
    path = "/api/v1/admin/rooms/gc/full-unbounded",
    params(
        ("limit" = Option<u32>, Query, description = "返回数量上限（默认 100，最大 1000）")
    ),
    responses(
        (status = 200, description = "查询成功", body = [FullRoomGcStatusView]),
        (status = 403, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn list_full_unbounded_rooms(
    headers: HeaderMap,
    Query(query): Query<AdminLimitQuery>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<FullRoomGcStatusView>> {
    ensure_admin(&headers)?;
    let limit = clamp_limit(query.limit);

    let rooms = app_state
        .services
        .room_lifecycle
        .list_full_unbounded_rooms(&app_state.connection_manager, limit)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list rooms for gc: {e}")))?;

    let result = rooms
        .into_iter()
        .map(|status| FullRoomGcStatusView {
            id: status.id,
            name: status.name,
            slug: status.slug,
            max_times_entered: status.max_times_entered,
            current_times_entered: status.current_times_entered,
            empty_since: status.empty_since,
            cleanup_after: status.cleanup_after,
            max_token_expires_at: status.max_token_expires_at,
            active_connections: u32::try_from(status.active_connections).unwrap_or(u32::MAX),
        })
        .collect();

    Ok(Json(result))
}

/// 触发一次 GC 扫描并清理到期房间（无活动连接才会实际清理）
#[utoipa::path(
    post,
    path = "/api/v1/admin/rooms/gc/run",
    params(
        ("limit" = Option<u32>, Query, description = "本次扫描数量上限（默认 100，最大 1000）")
    ),
    responses(
        (status = 200, description = "执行成功", body = RunRoomGcResponse),
        (status = 403, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn run_room_gc(
    headers: HeaderMap,
    Query(query): Query<AdminLimitQuery>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<RunRoomGcResponse> {
    ensure_admin(&headers)?;
    let limit = clamp_limit(query.limit);

    let report = app_state
        .services
        .room_lifecycle
        .run(
            &app_state.connection_manager,
            limit,
            app_state.config.room.share_disabled_lock_duration,
        )
        .await
        .map_err(|e| AppError::internal(format!("Failed to run room gc: {e}")))?;

    Ok(Json(RunRoomGcResponse {
        cleaned: u32::try_from(report.expired_rooms + report.full_rooms).unwrap_or(u32::MAX),
    }))
}

/// 平台 dashboard 统计概览
#[utoipa::path(
    get,
    path = "/api/v1/admin/stats",
    responses(
        (status = 200, description = "统计成功", body = AdminStatsResponse),
        (status = 403, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_stats(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<AdminStatsResponse> {
    ensure_admin(&headers)?;
    let repo = AdminConsoleRepository::new(app_state.db_pool.clone());

    let (rooms_total, rooms_open, rooms_protected) = repo
        .room_stats()
        .await
        .map_err(|e| AppError::internal(format!("Failed to aggregate rooms: {e}")))?;
    let (contents_total, contents_files, contents_messages, logical, physical, blob_count) = repo
        .content_stats()
        .await
        .map_err(|e| AppError::internal(format!("Failed to aggregate contents: {e}")))?;
    let metrics = app_state.connection_manager.get_metrics().await;

    Ok(Json(AdminStatsResponse {
        rooms_total,
        rooms_open,
        rooms_protected,
        contents_total,
        contents_files,
        contents_messages,
        storage_logical_bytes: logical,
        storage_physical_bytes: physical,
        blob_count,
        active_connections: u32::try_from(metrics.active_connections).unwrap_or(u32::MAX),
        active_rooms: u32::try_from(metrics.active_rooms).unwrap_or(u32::MAX),
    }))
}

/// 房间管理：列表 + 关键字搜索 + 分页
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminRoomListQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/rooms",
    params(
        ("q" = Option<String>, Query, description = "按 name / slug 模糊搜索"),
        ("limit" = Option<i64>, Query, description = "返回数量上限（默认 50，最大 500）"),
        ("offset" = Option<i64>, Query, description = "分页偏移")
    ),
    responses(
        (status = 200, description = "查询成功", body = AdminRoomListResponse),
        (status = 403, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_list_rooms(
    headers: HeaderMap,
    Query(query): Query<AdminRoomListQuery>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<AdminRoomListResponse> {
    ensure_admin(&headers)?;
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = AdminConsoleRepository::new(app_state.db_pool.clone());

    let total = repo
        .count_rooms(query.q.as_deref())
        .await
        .map_err(|e| AppError::internal(format!("Failed to count rooms: {e}")))?;
    let rooms = repo
        .list_rooms(query.q.as_deref(), limit, offset)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list rooms: {e}")))?
        .into_iter()
        .map(|row| AdminRoomView {
            id: row.id,
            name: row.name,
            slug: row.slug,
            status: row.status,
            password_protected: row.password_protected,
            current_size: row.current_size,
            max_size: row.max_size,
            current_times_entered: row.current_times_entered,
            max_times_entered: row.max_times_entered,
            expire_at: row.expire_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            content_count: row.content_count,
        })
        .collect();

    Ok(Json(AdminRoomListResponse {
        rooms,
        total,
        limit: u32::try_from(limit).unwrap_or(u32::MAX),
        offset: u32::try_from(offset).unwrap_or(0),
    }))
}

/// 房间管理：详情
#[utoipa::path(
    get,
    path = "/api/v1/admin/rooms/{name}",
    params(("name" = String, Path, description = "房间名称")),
    responses(
        (status = 200, description = "查询成功", body = AdminRoomDetailResponse),
        (status = 403, description = "未授权"),
        (status = 404, description = "房间不存在")
    ),
    tag = "admin"
)]
pub async fn admin_room_detail(
    headers: HeaderMap,
    AxumPath(name): AxumPath<String>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<AdminRoomDetailResponse> {
    ensure_admin(&headers)?;
    let room = RoomRepository::new(app_state.db_pool.clone())
        .find_by_name(&name)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::room_not_found(&name))?;
    let room_id = room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;

    let repo = AdminConsoleRepository::new(app_state.db_pool.clone());
    let (content_count, room_stats) = {
        let counts = repo
            .room_detail_counts(room_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to count room assets: {e}")))?;
        let all = repo
            .list_rooms(Some(&name), 1, 0)
            .await
            .map_err(|e| AppError::internal(format!("Failed to load room: {e}")))?;
        let view = all
            .into_iter()
            .find(|row| row.id == room_id)
            .ok_or_else(|| AppError::room_not_found(&name))?;
        (counts, view)
    };

    Ok(Json(AdminRoomDetailResponse {
        room: AdminRoomView {
            id: room_stats.id,
            name: room_stats.name,
            slug: room_stats.slug,
            status: room_stats.status,
            password_protected: room_stats.password_protected,
            current_size: room_stats.current_size,
            max_size: room_stats.max_size,
            current_times_entered: room_stats.current_times_entered,
            max_times_entered: room_stats.max_times_entered,
            expire_at: room_stats.expire_at,
            created_at: room_stats.created_at,
            updated_at: room_stats.updated_at,
            content_count: room_stats.content_count,
        },
        blob_count: content_count.0,
        token_count: content_count.1,
    }))
}

/// 房间管理：删除（平台运维语义，绕过房间 token；含存储与引用清理）
#[utoipa::path(
    delete,
    path = "/api/v1/admin/rooms/{name}",
    params(("name" = String, Path, description = "房间名称")),
    responses(
        (status = 200, description = "删除成功", body = DeleteRoomResponse),
        (status = 403, description = "未授权"),
        (status = 404, description = "房间不存在")
    ),
    tag = "admin"
)]
pub async fn admin_delete_room(
    headers: HeaderMap,
    AxumPath(name): AxumPath<String>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<DeleteRoomResponse> {
    ensure_admin(&headers)?;
    let room = RoomRepository::new(app_state.db_pool.clone())
        .find_by_name(&name)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::room_not_found(&name))?;
    let room_id = room
        .id
        .ok_or_else(|| AppError::internal("Room id missing"))?;

    let deleted = app_state
        .services
        .room_lifecycle
        .delete_room(&app_state.connection_manager, room_id, &room.slug)
        .await
        .map_err(|e| AppError::internal(format!("Failed to delete room: {e}")))?;
    if !deleted {
        return Err(AppError::room_not_found(name));
    }
    Ok(Json(DeleteRoomResponse {
        message: "Room deleted successfully".to_string(),
    }))
}

/// 存储状态（后端类型 / 用量 / 去重节省；不含凭据）
#[utoipa::path(
    get,
    path = "/api/v1/admin/storage",
    responses(
        (status = 200, description = "查询成功", body = AdminStorageResponse),
        (status = 403, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_storage(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<AdminStorageResponse> {
    ensure_admin(&headers)?;
    let repo = AdminConsoleRepository::new(app_state.db_pool.clone());
    let (_, _, _, logical, physical, blob_count) = repo
        .content_stats()
        .await
        .map_err(|e| AppError::internal(format!("Failed to aggregate storage: {e}")))?;

    let storage = &app_state.config.storage;
    let (backend, root, bucket) = match &storage.s3 {
        Some(s3) => ("s3", None, Some(s3.bucket.clone())),
        None => (
            "fs",
            Some(storage.root.to_string_lossy().into_owned()),
            None,
        ),
    };

    Ok(Json(AdminStorageResponse {
        backend: backend.to_string(),
        transfer_mode: match storage.transfer {
            crate::config::TransferMode::Proxy => "proxy",
            crate::config::TransferMode::Presigned => "presigned",
        }
        .to_string(),
        root,
        bucket,
        presign_base_url: storage.presign_base_url.clone(),
        physical_bytes: physical,
        logical_bytes: logical,
        blob_count,
        dedup_saved_bytes: (logical - physical).max(0),
    }))
}

/// 系统配置只读视图（机密不回显；运行时可写配置留待后续明确安全的小集合）
#[utoipa::path(
    get,
    path = "/api/v1/admin/config",
    responses(
        (status = 200, description = "查询成功", body = AdminConfigResponse),
        (status = 403, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_config(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<AdminConfigResponse> {
    ensure_admin(&headers)?;
    let config = &app_state.config;
    let storage = &config.storage;
    let admin_api_enabled = !std::env::var(ADMIN_TOKEN_ENV)
        .unwrap_or_default()
        .trim()
        .is_empty();

    Ok(Json(AdminConfigResponse {
        server_host: config.server.host.clone(),
        server_port: config.server.port,
        database_backend: if config.database.url.starts_with("postgres") {
            "postgresql"
        } else {
            "sqlite"
        }
        .to_string(),
        storage_backend: if storage.s3.is_some() { "s3" } else { "fs" }.to_string(),
        transfer_mode: match storage.transfer {
            crate::config::TransferMode::Proxy => "proxy",
            crate::config::TransferMode::Presigned => "presigned",
        }
        .to_string(),
        storage_root: storage.root.to_string_lossy().into_owned(),
        presign_base_url: storage.presign_base_url.clone(),
        room_default_max_size: config.room.defaults.max_content_size,
        room_default_max_times_entered: config.room.defaults.max_times_entered,
        room_default_role_key: config.room.defaults.default_role_key.clone(),
        jwt_ttl_seconds: config.auth.ttl_seconds,
        jwt_refresh_ttl_seconds: config.auth.refresh_ttl_seconds,
        upload_reservation_ttl_seconds: storage.upload_reservation_ttl_seconds,
        admin_api_enabled,
    }))
}
