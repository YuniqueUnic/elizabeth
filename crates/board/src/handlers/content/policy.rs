use std::sync::Arc;

use axum::extract::{Path as AxumPath, State};
use axum::Json;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::{Utc, Duration};

use crate::errors::AppError;
use crate::handlers::{AuthToken, verify_room_token};
use crate::models::permission::RoomPermission;
use crate::repository::{
    DownloadPolicyRepository, IDownloadPolicyRepository, IRoomContentRepository,
    RoomContentRepository,
};
use crate::state::AppState;
use crate::validation::{RoomNameValidator, TokenValidator};
use board_protocol::models::room::{DownloadPolicyMode, FileAccessCode, FileDownloadPolicy};

use super::{ContentPermission, ensure_permission};

#[utoipa::path(
    get,
    path = "/api/v1/rooms/{name}/contents/{content_id}/policy",
    params(
        ("name" = String, Path, description = "房间名称"),
        ("content_id" = i64, Path, description = "内容 id"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    responses(
        (status = 200, description = "策略详情", body = PolicyResponse),
        (status = 204, description = "未设置策略"),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无访问权限"),
        (status = 404, description = "文件不存在")
    ),
    tag = "content"
)]
pub async fn get_policy(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Option<PolicyResponse>>, AppError> {
    RoomNameValidator::validate_identifier(&name)?;
    TokenValidator::validate_token_format(&token)?;
    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    let content_repo = RoomContentRepository::new(app_state.db_pool.clone());
    let content = content_repo
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    if content.room_id != verified.room.id.unwrap() {
        return Err(AppError::not_found("Content not found in this room"));
    }

    let policy_repo = DownloadPolicyRepository::new(app_state.db_pool.clone());
    let policy = policy_repo
        .get_policy_by_content_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {}", e)))?;

    Ok(Json(policy.map(|p| PolicyResponse {
        id: p.id.unwrap(),
        content_id: p.content_id,
        mode: p.mode,
        max_downloads: p.max_downloads,
        download_count: p.download_count,
    })))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetPolicyRequest {
    pub mode: DownloadPolicyMode,
    pub max_downloads: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PolicyResponse {
    pub id: i64,
    pub content_id: i64,
    pub mode: DownloadPolicyMode,
    pub max_downloads: Option<i64>,
    pub download_count: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GenerateCodesRequest {
    pub count: i32,
    pub is_reusable: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GenerateCodesResponse {
    pub codes: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RedeemRequest {
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RedeemResponse {
    pub ticket: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadTicketClaims {
    pub sub: String,
    pub exp: usize,
    pub content_id: i64,
}

#[utoipa::path(
    put,
    path = "/api/v1/rooms/{name}/contents/{content_id}/policy",
    request_body = SetPolicyRequest,
    params(
        ("name" = String, Path, description = "房间名称"),
        ("content_id" = i64, Path, description = "内容 id"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    responses(
        (status = 200, description = "更新成功", body = PolicyResponse),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无访问权限"),
        (status = 404, description = "文件不存在")
    ),
    tag = "content"
)]
pub async fn set_policy(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SetPolicyRequest>,
) -> Result<Json<PolicyResponse>, AppError> {
    TokenValidator::validate_token_format(&token)?;
    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Delete,
    )?;

    let content_repo = RoomContentRepository::new(app_state.db_pool.clone());
    let content = content_repo
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    if content.room_id != verified.room.id.unwrap() {
        return Err(AppError::not_found("Content not found in this room"));
    }

    let policy_repo = DownloadPolicyRepository::new(app_state.db_pool.clone());
    
    let now = chrono::Utc::now().naive_utc();
    let policy = FileDownloadPolicy {
        id: None,
        content_id,
        mode: payload.mode,
        max_downloads: payload.max_downloads,
        download_count: 0,
        created_at: now,
        updated_at: now,
    };

    let saved = policy_repo.upsert_policy(&policy).await
        .map_err(|e| AppError::internal(format!("Failed to save policy: {}", e)))?;

    Ok(Json(PolicyResponse {
        id: saved.id.unwrap(),
        content_id: saved.content_id,
        mode: saved.mode,
        max_downloads: saved.max_downloads,
        download_count: saved.download_count,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/contents/{content_id}/policy/generate-codes",
    request_body = GenerateCodesRequest,
    params(
        ("name" = String, Path, description = "房间名称"),
        ("content_id" = i64, Path, description = "内容 id"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    responses(
        (status = 200, description = "生成成功", body = GenerateCodesResponse),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无访问权限"),
        (status = 404, description = "文件/策略不存在")
    ),
    tag = "content"
)]
pub async fn generate_codes(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<GenerateCodesRequest>,
) -> Result<Json<GenerateCodesResponse>, AppError> {
    if payload.count <= 0 || payload.count > 1000 {
        return Err(AppError::validation("Count must be between 1 and 1000"));
    }

    TokenValidator::validate_token_format(&token)?;
    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Delete,
    )?;

    let content_repo = RoomContentRepository::new(app_state.db_pool.clone());
    let content = content_repo
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    if content.room_id != verified.room.id.unwrap() {
        return Err(AppError::not_found("Content not found in this room"));
    }

    let policy_repo = DownloadPolicyRepository::new(app_state.db_pool.clone());
    let policy = policy_repo.get_policy_by_content_id(content_id).await
        .map_err(|e| AppError::internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| AppError::not_found("Policy not found for this content"))?;

    let policy_id = policy.id.unwrap();
    let mut clear_codes = Vec::new();
    let mut db_codes = Vec::new();
    let now = chrono::Utc::now().naive_utc();

    for _ in 0..payload.count {
        let code = Uuid::new_v4().to_string();
        clear_codes.push(code.clone());
        
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        let hash = hex::encode(hasher.finalize());

        db_codes.push(FileAccessCode {
            id: None,
            policy_id,
            code_hash: hash,
            is_reusable: payload.is_reusable,
            used_at: None,
            created_at: now,
        });
    }

    policy_repo.create_access_codes(&db_codes).await
        .map_err(|e| AppError::internal(format!("Failed to create codes: {}", e)))?;

    Ok(Json(GenerateCodesResponse { codes: clear_codes }))
}

#[utoipa::path(
    post,
    path = "/api/v1/rooms/{name}/contents/{content_id}/redeem",
    request_body = RedeemRequest,
    params(
        ("name" = String, Path, description = "房间名称"),
        ("content_id" = i64, Path, description = "内容 id"),
        ("token" = String, Query, description = "有效的房间 token")
    ),
    responses(
        (status = 200, description = "兑换成功", body = RedeemResponse),
        (status = 401, description = "token 无效"),
        (status = 403, description = "无访问权限或无效兑换码"),
        (status = 404, description = "文件/策略不存在")
    ),
    tag = "content"
)]
pub async fn redeem_code(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    AuthToken(token): AuthToken,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<RedeemRequest>,
) -> Result<Json<RedeemResponse>, AppError> {
    TokenValidator::validate_token_format(&token)?;
    let verified = verify_room_token(app_state.clone(), &name, &token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    let content_repo = RoomContentRepository::new(app_state.db_pool.clone());
    let content = content_repo
        .find_by_id(content_id)
        .await
        .map_err(|e| AppError::internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| AppError::not_found("Content not found"))?;

    if content.room_id != verified.room.id.unwrap() {
        return Err(AppError::not_found("Content not found in this room"));
    }

    let policy_repo = DownloadPolicyRepository::new(app_state.db_pool.clone());
    
    let mut hasher = Sha256::new();
    hasher.update(payload.code.as_bytes());
    let code_hash = hex::encode(hasher.finalize());

    let redeemed = policy_repo.redeem_code(content_id, &code_hash).await
        .map_err(|e| AppError::internal(format!("Failed to redeem code: {}", e)))?;

    if !redeemed {
        return Err(AppError::authorization("Invalid, expired, or depleted code"));
    }

    // Generate ticket
    let exp = Utc::now() + Duration::seconds(120);
    let claims = DownloadTicketClaims {
        sub: "download_ticket".to_string(),
        exp: exp.timestamp() as usize,
        content_id,
    };

    let ticket = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(app_state.config.auth.jwt_secret.as_bytes()),
    ).map_err(|_| AppError::internal("Failed to generate ticket"))?;

    Ok(Json(RedeemResponse { ticket }))
}
