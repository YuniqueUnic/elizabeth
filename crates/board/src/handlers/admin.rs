use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::dto::{FullRoomGcStatusView, RunRoomGcResponse};
use crate::errors::{AppError, AppResult};
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

fn ensure_admin(headers: &HeaderMap) -> AppResult<()> {
    let expected = std::env::var(ADMIN_TOKEN_ENV).unwrap_or_default();
    if expected.trim().is_empty() {
        return Err(AppError::authorization(format!(
            "Admin API disabled (set {ADMIN_TOKEN_ENV})"
        )));
    }

    let provided = headers
        .get(ADMIN_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected {
        return Err(AppError::authorization("Invalid admin token"));
    }

    Ok(())
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
        .room_gc
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

    let cleaned = app_state
        .services
        .room_gc
        .run_scheduled_gc(&app_state.connection_manager, limit)
        .await
        .map_err(|e| AppError::internal(format!("Failed to run room gc: {e}")))?;

    Ok(Json(RunRoomGcResponse {
        cleaned: u32::try_from(cleaned).unwrap_or(u32::MAX),
    }))
}
