use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Deserialize;

use crate::authz::{Authz, Resource};
use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token_by_id};
use crate::models::content::RoomContent;
use crate::models::room::role::Capability;
use crate::repository::{
    DownloadPolicyRepository, IDownloadPolicyRepository, IRoomContentRepository,
    RoomContentRepository,
};
use crate::state::AppState;
use crate::validation::TokenValidator;
use board_protocol::models::room::DownloadPolicyMode;

use super::policy::DownloadTicketClaims;

#[derive(Deserialize)]
pub struct DownloadQuery {
    pub ticket: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/contents/{content_id}",
    params(
        ("content_id" = i64, Path, description = "内容 id"),
        ("token" = String, Query, description = "有效的房间 token"),
        ("ticket" = Option<String>, Query, description = "下载票据，当文件策略非 off 时必需")
    ),
    responses(
        (status = 200, description = "文件内容"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无访问权限或无效票据"),
        (status = 404, description = "文件不存在")
    ),
    tag = "content"
)]
pub async fn download_content_global(
    AxumPath(content_id): AxumPath<i64>,
    Query(query): Query<DownloadQuery>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> Result<Response, AppError> {
    TokenValidator::validate_token_format(&token)?;

    let repository = RoomContentRepository::new(app_state.db_pool.clone());
    let content = repository
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {e}")))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    let verified = verify_room_token_by_id(app_state.clone(), content.room_id, &token).await?;
    let authz = Authz::for_claims(&app_state, &verified.room, &verified.claims).await?;
    authz.require(
        Capability::FileDownload,
        &Resource::Content {
            room_id: content.room_id,
            content_type: content.content_type,
            created_by_jti: content.created_by_jti.as_deref(),
        },
    )?;

    // 隐藏文件对外按不存在处理，堵住直链绕过列表过滤的口子。
    super::visibility::ensure_content_visible(&authz, &content)?;

    // Check policy
    let policy_repo = DownloadPolicyRepository::new(app_state.db_pool.clone());
    let policy = policy_repo
        .get_policy_by_content_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to query policy: {e}")))?;

    if let Some(p) = policy {
        if p.mode != DownloadPolicyMode::Off {
            if p.mode == DownloadPolicyMode::Reusable
                && let Some(max_dl) = p.max_downloads
                && p.download_count >= max_dl
            {
                return Err(AppError::authorization("Max download limit reached"));
            }

            let ticket = query
                .ticket
                .ok_or_else(|| AppError::authorization("Download ticket required"))?;
            let mut validation = Validation::new(Algorithm::HS256);
            validation.validate_exp = true;

            let token_data = decode::<DownloadTicketClaims>(
                &ticket,
                &DecodingKey::from_secret(app_state.config.auth.jwt_secret.as_bytes()),
                &validation,
            )
            .map_err(|_| AppError::authorization("Invalid or expired download ticket"))?;

            if token_data.claims.content_id != content_id {
                return Err(AppError::authorization("Ticket is not for this content"));
            }

            // Increment download count
            let _ = policy_repo.increment_download_count(content_id).await;
        } else if let Some(max_dl) = p.max_downloads {
            if p.download_count >= max_dl {
                return Err(AppError::authorization("Max download limit reached"));
            }
            let _ = policy_repo.increment_download_count(content_id).await;
        }
    }

    // presigned 传输模式：鉴权（token / 票据 / 策略）通过后签发短时效直下 URL。
    // 历史本地内容（FS locator）无法预签名，回落到代理流式传输。
    if app_state.transfer_mode() == crate::config::TransferMode::Presigned
        && let Some(locator) = content.path.as_deref()
    {
        match app_state
            .storage
            .presign_read(locator, app_state.presign_ttl())
            .await
        {
            Ok(url) => {
                return Ok(axum::response::Redirect::temporary(&url).into_response());
            }
            Err(crate::storage::StorageError::Unsupported) => {}
            Err(error) => {
                return Err(AppError::internal(format!(
                    "Presign download failed: {error}"
                )));
            }
        }
    }

    serve_content_stream(&app_state, content).await
}

async fn serve_content_stream(
    app_state: &Arc<AppState>,
    content: RoomContent,
) -> Result<Response, AppError> {
    let locator = content
        .path
        .ok_or_else(|| AppError::not_found("Content not stored on disk"))?;

    let file_name = content.file_name.clone().unwrap_or_else(|| {
        locator
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("download.bin")
            .to_string()
    });

    let stream = app_state.storage.read(&locator).await.map_err(|error| {
        if matches!(error, crate::storage::StorageError::NotFound(_)) {
            AppError::not_found("File missing on disk")
        } else {
            AppError::internal(format!("Read content failed: {error}"))
        }
    })?;
    let body = Body::from_stream(stream);
    let mut response = Response::new(body);
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
        .map_err(|_| AppError::internal("Failed to build response headers"))?;
    response
        .headers_mut()
        .insert(CONTENT_DISPOSITION, disposition);

    if let Some(size) = content.size
        && let Ok(value) = HeaderValue::from_str(&size.to_string())
    {
        response.headers_mut().insert(CONTENT_LENGTH, value);
    }

    if let Some(mime) = content.mime_type
        && let Ok(value) = HeaderValue::from_str(&mime)
    {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }

    Ok(response)
}
