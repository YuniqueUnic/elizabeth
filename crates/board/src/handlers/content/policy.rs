use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

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

    if let Some(p) = policy {
        let (total_codes, remaining_codes, used_codes) = policy_repo
            .get_code_stats(p.id.unwrap())
            .await
            .unwrap_or((0, 0, 0));

        Ok(Json(Some(PolicyResponse {
            id: p.id.unwrap(),
            content_id: p.content_id,
            mode: p.mode,
            max_downloads: p.max_downloads,
            download_count: p.download_count,
            total_codes,
            remaining_codes,
            used_codes,
        })))
    } else {
        Ok(Json(None))
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetPolicyRequest {
    pub mode: DownloadPolicyMode,
    pub max_downloads: Option<i64>,
    pub codes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PolicyResponse {
    pub id: i64,
    pub content_id: i64,
    pub mode: DownloadPolicyMode,
    pub max_downloads: Option<i64>,
    pub download_count: i64,
    pub total_codes: i64,
    pub remaining_codes: i64,
    pub used_codes: i64,
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

    let mut final_max_downloads = payload.max_downloads;
    let mut new_codes: Option<Vec<FileAccessCode>> = None;
    let mut should_clear_codes = false;

    match payload.mode {
        DownloadPolicyMode::Off => {
            should_clear_codes = true;
        }
        DownloadPolicyMode::Reusable => {
            if let Some(codes) = payload.codes {
                let valid_codes: Vec<String> =
                    codes.into_iter().filter(|c| !c.trim().is_empty()).collect();
                if valid_codes.len() > 1 {
                    return Err(AppError::validation(
                        "Reusable mode accepts at most 1 access code",
                    ));
                }

                if !valid_codes.is_empty() {
                    let mut hasher = Sha256::new();
                    hasher.update(valid_codes[0].as_bytes());
                    let code_hash = hex::encode(hasher.finalize());

                    new_codes = Some(vec![FileAccessCode {
                        id: None,
                        policy_id: 0,
                        code_hash,
                        is_reusable: true,
                        used_at: None,
                        created_at: Utc::now().naive_utc(),
                    }]);
                }
            }
        }
        DownloadPolicyMode::OneTime => {
            if let Some(codes) = payload.codes {
                let mut unique_codes: Vec<String> =
                    codes.into_iter().filter(|c| !c.trim().is_empty()).collect();
                unique_codes.sort();
                unique_codes.dedup();

                if unique_codes.len() > 1000 {
                    return Err(AppError::validation(
                        "Cannot set more than 1000 access codes at once",
                    ));
                }

                let mut to_insert = Vec::with_capacity(unique_codes.len());
                let now = Utc::now().naive_utc();
                for code in &unique_codes {
                    let mut hasher = Sha256::new();
                    hasher.update(code.as_bytes());
                    let code_hash = hex::encode(hasher.finalize());

                    to_insert.push(FileAccessCode {
                        id: None,
                        policy_id: 0,
                        code_hash,
                        is_reusable: false,
                        used_at: None,
                        created_at: now,
                    });
                }

                final_max_downloads = Some(unique_codes.len() as i64);
                new_codes = Some(to_insert);
            }
        }
    }

    let now = chrono::Utc::now().naive_utc();
    let policy = FileDownloadPolicy {
        id: None,
        content_id,
        mode: payload.mode,
        max_downloads: final_max_downloads,
        download_count: 0,
        created_at: now,
        updated_at: now,
    };

    let saved = policy_repo
        .upsert_policy(&policy)
        .await
        .map_err(|e| AppError::internal(format!("Failed to save policy: {}", e)))?;

    let policy_id = saved.id.unwrap();

    if should_clear_codes {
        policy_repo
            .clear_access_codes(policy_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to clear access codes: {}", e)))?;
    } else if let Some(mut codes_to_insert) = new_codes {
        for c in &mut codes_to_insert {
            c.policy_id = policy_id;
        }
        policy_repo
            .replace_access_codes(policy_id, &codes_to_insert)
            .await
            .map_err(|e| AppError::internal(format!("Failed to replace access codes: {}", e)))?;
    }

    let (total_codes, remaining_codes, used_codes) = policy_repo
        .get_code_stats(policy_id)
        .await
        .unwrap_or((0, 0, 0));

    Ok(Json(PolicyResponse {
        id: saved.id.unwrap(),
        content_id: saved.content_id,
        mode: saved.mode,
        max_downloads: saved.max_downloads,
        download_count: saved.download_count,
        total_codes,
        remaining_codes,
        used_codes,
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
    let policy = policy_repo
        .get_policy_by_content_id(content_id)
        .await
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

    policy_repo
        .create_access_codes(&db_codes)
        .await
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
    headers: HeaderMap,
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

    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("client");

    let client_key = format!("{}:{}", client_ip, &verified.claims.jti);

    // 1. Check anti-brute-force rate limit before processing
    app_state
        .access_code_limiter()
        .check_rate_limit(content_id, &client_key)?;

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

    let code = payload.code.trim();
    if code.is_empty() {
        return Err(AppError::validation("Access code cannot be empty"));
    }

    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    let code_hash = hex::encode(hasher.finalize());

    let redeemed = policy_repo
        .redeem_code(content_id, &code_hash)
        .await
        .map_err(|e| AppError::internal(format!("Failed to redeem code: {}", e)))?;

    if !redeemed {
        let (failed_count, lockout_secs) = app_state
            .access_code_limiter()
            .record_failure(content_id, &client_key);
        if let Some(secs) = lockout_secs {
            return Err(AppError::too_many_requests(format!(
                "Too many failed attempts ({} attempts). Access temporarily locked for {} seconds.",
                failed_count, secs
            )));
        } else {
            let remaining_tries = 5u32.saturating_sub(failed_count);
            return Err(AppError::authorization(format!(
                "Invalid, expired, or depleted code. {} attempts remaining before lockout.",
                remaining_tries
            )));
        }
    }

    // Success -> clear failed attempts
    app_state
        .access_code_limiter()
        .record_success(content_id, &client_key);

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
    )
    .map_err(|_| AppError::internal("Failed to generate ticket"))?;

    Ok(Json(RedeemResponse { ticket }))
}
