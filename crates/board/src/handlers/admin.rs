use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::HeaderMap;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::dto::{
    AdminConfigResponse, AdminCredentialUpdateRequest, AdminCredentialView,
    AdminMintIdentityCodeRequest, AdminRoomDetailResponse, AdminRoomListResponse, AdminRoomView,
    AdminStatsResponse, AdminStorageResponse, CreateRoomIdentityCodeResponse, DeleteRoomResponse,
    FullRoomGcStatusView, RoomExpiryOverride, RunRoomGcResponse, UpdateRoomSettingsRequest,
    UpdateRuntimeConfigRequest,
};
use crate::errors::{AppError, AppResult};
use crate::handlers::rooms::identity_codes::validate_identity_code;
use crate::repository::{AdminConsoleRepository, IRoomRepository, RoomRepository};
use crate::services::GuardScope;
use crate::state::AppState;

type HandlerResult<T> = Result<Json<T>, AppError>;

const DEFAULT_ADMIN_LIMIT: u32 = 100;
const MAX_ADMIN_LIMIT: u32 = 1000;
const ADMIN_TOKEN_HEADER: &str = "X-Elizabeth-Admin-Token";

#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminLimitQuery {
    pub limit: Option<u32>,
}

/// 管理凭证校验 + 防爆破：失败按客户端累计，超阈值锁定（与房间密码/身份码同一守卫）。
/// 引导补签（issue_token）等内部路径也复用本函数；未携带管理头的请求
/// （普通用户流）只做凭证判定，不计入防爆破失败。
pub(crate) fn ensure_admin(app_state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    let Some(provided) = headers
        .get(ADMIN_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return app_state.admin_credential.verify(None);
    };

    let client = crate::client_ip(headers);
    app_state
        .attempt_guard()
        .check(GuardScope::AdminLogin, client)?;

    match app_state.admin_credential.verify(Some(provided)) {
        Ok(()) => {
            app_state
                .attempt_guard()
                .record_success(GuardScope::AdminLogin, client);
            Ok(())
        }
        Err(error) => {
            app_state
                .attempt_guard()
                .record_failure(GuardScope::AdminLogin, client);
            Err(error)
        }
    }
}

/// 管理 API 是否可用：配置了环境引导凭证或运行时轮换凭证即视为启用。
fn admin_api_enabled(app_state: &AppState) -> bool {
    app_state.admin_credential.source() == "runtime-override"
        || !std::env::var(crate::state::ADMIN_TOKEN_ENV)
            .unwrap_or_default()
            .trim()
            .is_empty()
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
    ensure_admin(&app_state, &headers)?;
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
    ensure_admin(&app_state, &headers)?;
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
    ensure_admin(&app_state, &headers)?;
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
    ensure_admin(&app_state, &headers)?;
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
            default_role_key: row.default_role_key,
            upload_file_type: row.upload_file_type,
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
    ensure_admin(&app_state, &headers)?;
    Ok(Json(room_detail_response(&app_state, &name).await?))
}

async fn room_detail_response(
    app_state: &AppState,
    name: &str,
) -> AppResult<AdminRoomDetailResponse> {
    let room = RoomRepository::new(app_state.db_pool.clone())
        .find_by_name(name)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::room_not_found(name))?;
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
            .list_rooms(Some(name), 1, 0)
            .await
            .map_err(|e| AppError::internal(format!("Failed to load room: {e}")))?;
        let view = all
            .into_iter()
            .find(|row| row.id == room_id)
            .ok_or_else(|| AppError::room_not_found(name))?;
        (counts, view)
    };

    Ok(AdminRoomDetailResponse {
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
            default_role_key: room_stats.default_role_key,
            upload_file_type: room_stats.upload_file_type,
            expire_at: room_stats.expire_at,
            created_at: room_stats.created_at,
            updated_at: room_stats.updated_at,
            content_count: room_stats.content_count,
        },
        blob_count: content_count.0,
        token_count: content_count.1,
    })
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
    ensure_admin(&app_state, &headers)?;
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
    ensure_admin(&app_state, &headers)?;
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
    ensure_admin(&app_state, &headers)?;
    Ok(Json(build_admin_config_response(&app_state)))
}

fn build_admin_config_response(app_state: &AppState) -> AdminConfigResponse {
    let config = &app_state.config;
    let storage = &config.storage;
    AdminConfigResponse {
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
        room_expiry_allowed_ages_seconds: config.room.expiry.allowed_ages_seconds().to_vec(),
        room_expiry_default_age_seconds: config.room.expiry.default_age_seconds(),
        jwt_ttl_seconds: config.auth.ttl_seconds,
        jwt_refresh_ttl_seconds: config.auth.refresh_ttl_seconds,
        upload_reservation_ttl_seconds: storage.upload_reservation_ttl_seconds,
        admin_api_enabled: admin_api_enabled(app_state),
        dedup_scope: if storage.global_dedup {
            "global"
        } else {
            "per-room"
        }
        .to_string(),
        runtime_disallow_search_indexing: app_state.runtime.disallow_search_indexing(),
        runtime_room_default_max_size: app_state.runtime.room_default_max_size(),
        runtime_room_default_max_times_entered: app_state.runtime.room_default_max_times_entered(),
        runtime_session_ttl_seconds: app_state.runtime.session_ttl_seconds(),
        runtime_upload_reservation_ttl_seconds: app_state.runtime.upload_reservation_ttl_seconds(),
        runtime_room_default_role_key: app_state.runtime.room_default_role_key(),
        runtime_room_expiry: app_state.runtime.room_expiry_policy().map(|policy| {
            crate::dto::admin::RoomExpiryOverride {
                allowed_ages_seconds: policy.allowed_ages_seconds().to_vec(),
                default_age_seconds: policy.default_age_seconds(),
            }
        }),
        admin_token_source: app_state.admin_credential.source().to_string(),
    }
}

/// 运行时可写配置更新（白名单：robots 开关、新房间默认容量/进入次数/角色、
/// 房间有效期策略、会话有效期、上传预留有效期）。覆盖仅存活于进程内，重启回退到配置文件值；
/// 数值字段传 0 表示清除覆盖，角色字段传空串表示清除覆盖，有效期策略传空允许列表表示清除覆盖。
#[utoipa::path(
    put,
    path = "/api/v1/admin/config/runtime",
    request_body = UpdateRuntimeConfigRequest,
    responses(
        (status = 200, description = "更新成功，返回当前生效配置", body = AdminConfigResponse),
        (status = 400, description = "参数不合法"),
        (status = 403, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_update_runtime_config(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRuntimeConfigRequest>,
) -> HandlerResult<AdminConfigResponse> {
    ensure_admin(&app_state, &headers)?;

    const MIN_ROOM_MAX_SIZE: i64 = 1024 * 1024; // 1 MiB
    const MAX_ROOM_MAX_SIZE: i64 = 1024 * 1024 * 1024 * 1024; // 1 TiB
    const MAX_ROOM_TIMES_ENTERED: i64 = 1_000_000_000;
    const MIN_SESSION_TTL: i64 = 300; // 5 min
    const MAX_SESSION_TTL: i64 = 7 * 24 * 3600;
    const MIN_RESERVATION_TTL: i64 = 60;
    const MAX_RESERVATION_TTL: i64 = 24 * 3600;

    if let Some(size) = payload.room_default_max_size {
        if size != 0 && !(MIN_ROOM_MAX_SIZE..=MAX_ROOM_MAX_SIZE).contains(&size) {
            return Err(AppError::validation(format!(
                "room_default_max_size must be 0 (reset) or within {MIN_ROOM_MAX_SIZE}..{MAX_ROOM_MAX_SIZE}"
            )));
        }
        app_state.runtime.set_room_default_max_size(size);
    }
    if let Some(times) = payload.room_default_max_times_entered {
        if times != 0 && !(1..=MAX_ROOM_TIMES_ENTERED).contains(&times) {
            return Err(AppError::validation(format!(
                "room_default_max_times_entered must be 0 (reset) or within 1..{MAX_ROOM_TIMES_ENTERED}"
            )));
        }
        app_state.runtime.set_room_default_max_times_entered(times);
    }
    if let Some(ttl) = payload.session_ttl_seconds {
        if ttl != 0 && !(MIN_SESSION_TTL..=MAX_SESSION_TTL).contains(&ttl) {
            return Err(AppError::validation(format!(
                "session_ttl_seconds must be 0 (reset) or within {MIN_SESSION_TTL}..{MAX_SESSION_TTL}"
            )));
        }
        app_state.runtime.set_session_ttl_seconds(ttl);
    }
    if let Some(ttl) = payload.upload_reservation_ttl_seconds {
        if ttl != 0 && !(MIN_RESERVATION_TTL..=MAX_RESERVATION_TTL).contains(&ttl) {
            return Err(AppError::validation(format!(
                "upload_reservation_ttl_seconds must be 0 (reset) or within {MIN_RESERVATION_TTL}..{MAX_RESERVATION_TTL}"
            )));
        }
        app_state.runtime.set_upload_reservation_ttl_seconds(ttl);
    }
    if let Some(role) = payload.room_default_role_key {
        let trimmed = role.trim();
        if trimmed.is_empty() {
            app_state.runtime.set_room_default_role_key(None);
        } else if !board_protocol::models::room::role::is_system_role_key(trimmed) {
            return Err(AppError::validation(
                "room_default_role_key must be one of the system roles: admin, editor, reader",
            ));
        } else {
            app_state
                .runtime
                .set_room_default_role_key(Some(trimmed.to_owned()));
        }
    }
    if let Some(disallow) = payload.disallow_search_indexing {
        app_state.runtime.set_disallow_search_indexing(disallow);
    }
    if let Some(expiry) = payload.room_expiry {
        // 允许列表与默认时长互相约束，因此整组校验后原子替换。
        let policy = validated_room_expiry_override(expiry)?;
        app_state
            .runtime
            .set_room_expiry_policy(policy.map(Arc::new));
    }

    Ok(Json(build_admin_config_response(&app_state)))
}

/// 校验并生成房间有效期策略覆盖；空允许列表 = 清除覆盖。
pub(crate) fn validated_room_expiry_override(
    expiry: RoomExpiryOverride,
) -> AppResult<Option<crate::config::RoomExpiryPolicy>> {
    if expiry.allowed_ages_seconds.is_empty() {
        return Ok(None);
    }
    crate::config::RoomExpiryPolicy::new(expiry.allowed_ages_seconds, expiry.default_age_seconds)
        .map(Some)
        .map_err(|error| AppError::validation(format!("Invalid room expiry policy: {error}")))
}

/// 轮换平台管理 API 凭证（进程内覆盖，重启回退环境引导值；凭证不回显）。
#[utoipa::path(
    put,
    path = "/api/v1/admin/credential",
    request_body = AdminCredentialUpdateRequest,
    responses(
        (status = 200, description = "轮换成功", body = AdminCredentialView),
        (status = 400, description = "凭证强度不足"),
        (status = 403, description = "未授权")
    ),
    tag = "admin"
)]
pub async fn admin_update_credential(
    headers: HeaderMap,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<AdminCredentialUpdateRequest>,
) -> HandlerResult<AdminCredentialView> {
    ensure_admin(&app_state, &headers)?;

    const MIN_ADMIN_TOKEN_LEN: usize = 12;
    let token = payload.token.trim();
    if token.len() < MIN_ADMIN_TOKEN_LEN || token.chars().any(char::is_whitespace) {
        return Err(AppError::validation(format!(
            "admin token must be at least {MIN_ADMIN_TOKEN_LEN} characters without whitespace"
        )));
    }
    app_state.admin_credential.rotate(token.to_owned());

    Ok(Json(AdminCredentialView {
        admin_token_source: app_state.admin_credential.source().to_string(),
    }))
}

/// 房间管理：更新房间设置（平台运维语义；复用房间设置校验与落库核心）。
#[utoipa::path(
    put,
    path = "/api/v1/admin/rooms/{name}",
    params(("name" = String, Path, description = "房间名称")),
    request_body = UpdateRoomSettingsRequest,
    responses(
        (status = 200, description = "更新成功", body = AdminRoomDetailResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "未授权"),
        (status = 404, description = "房间不存在")
    ),
    tag = "admin"
)]
pub async fn admin_update_room(
    headers: HeaderMap,
    AxumPath(name): AxumPath<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRoomSettingsRequest>,
) -> HandlerResult<AdminRoomDetailResponse> {
    ensure_admin(&app_state, &headers)?;
    let room = RoomRepository::new(app_state.db_pool.clone())
        .find_by_name(&name)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::room_not_found(&name))?;
    crate::handlers::rooms::settings::apply_room_settings_update(&app_state, room, payload).await?;
    Ok(Json(room_detail_response(&app_state, &name).await?))
}

/// 房间管理：铸造房间身份码（code 缺省时服务端生成；admin 角色跟随房间过期）。
#[utoipa::path(
    post,
    path = "/api/v1/admin/rooms/{name}/identity-codes",
    params(("name" = String, Path, description = "房间名称")),
    request_body = AdminMintIdentityCodeRequest,
    responses(
        (status = 200, description = "铸造成功，明文 code 仅此一次返回", body = CreateRoomIdentityCodeResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "未授权"),
        (status = 404, description = "房间不存在")
    ),
    tag = "admin"
)]
pub async fn admin_mint_identity_code(
    headers: HeaderMap,
    AxumPath(name): AxumPath<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<AdminMintIdentityCodeRequest>,
) -> HandlerResult<CreateRoomIdentityCodeResponse> {
    ensure_admin(&app_state, &headers)?;
    let room = RoomRepository::new(app_state.db_pool.clone())
        .find_by_name(&name)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {e}")))?
        .ok_or_else(|| AppError::room_not_found(&name))?;
    let requested_code = match payload.code.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(code) => Some(validate_identity_code(code)?),
    };
    let (identity_code, code) = crate::handlers::rooms::identity_codes::mint_identity_code(
        &app_state,
        &room,
        &payload.role,
        requested_code,
        payload.expires_in_secs,
        None,
    )
    .await?;
    Ok(Json(CreateRoomIdentityCodeResponse {
        identity_code,
        code,
    }))
}
