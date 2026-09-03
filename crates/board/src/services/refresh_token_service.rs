use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::authz::{RoleTableCache, load_role_table};
use crate::db::DbPool;
use crate::models::{Grant, RefreshTokenResponse, Room, RoomRefreshToken, RoomStatus, RoomToken};
use crate::repository::room_access_repository::RoomAccessRepository;
use crate::repository::room_refresh_token_repository::{
    IRoomRefreshTokenRepository, ITokenBlacklistRepository,
};
use crate::repository::room_repository::{IRoomRepository, RoomRepository};
use crate::repository::room_token_repository::{IRoomTokenRepository, RoomTokenRepository};
use crate::services::token::{RoomTokenClaims, RoomTokenService};

#[derive(Debug)]
pub struct PreparedRefreshToken {
    pub signed_token: String,
    pub expires_at: chrono::NaiveDateTime,
    pub record: RoomRefreshToken,
}

#[derive(Clone)]
pub struct RefreshTokenService {
    base_service: RoomTokenService,
    refresh_ttl: Duration,
    enable_rotation: bool,
    db_pool: Arc<DbPool>,
    roles_cache: Arc<RoleTableCache>,
    room_repository: Arc<RoomRepository>,
    access_repository: RoomAccessRepository,
    access_token_repository: Arc<RoomTokenRepository>,
    refresh_token_repository: Arc<dyn IRoomRefreshTokenRepository + Send + Sync>,
    blacklist_repository: Arc<dyn ITokenBlacklistRepository + Send + Sync>,
}

impl RefreshTokenService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_service: RoomTokenService,
        refresh_ttl: Duration,
        enable_rotation: bool,
        db_pool: Arc<DbPool>,
        roles_cache: Arc<RoleTableCache>,
        room_repository: Arc<RoomRepository>,
        access_repository: RoomAccessRepository,
        access_token_repository: Arc<RoomTokenRepository>,
        refresh_token_repository: Arc<dyn IRoomRefreshTokenRepository + Send + Sync>,
        blacklist_repository: Arc<dyn ITokenBlacklistRepository + Send + Sync>,
    ) -> Self {
        Self {
            base_service,
            refresh_ttl,
            enable_rotation,
            db_pool,
            roles_cache,
            room_repository,
            access_repository,
            access_token_repository,
            refresh_token_repository,
            blacklist_repository,
        }
    }

    pub fn prepare_refresh_token(
        &self,
        room: &Room,
        role_key: &str,
        access_jti: impl Into<String>,
    ) -> Result<PreparedRefreshToken> {
        ensure_room_open(room)?;
        let room_id = room.id.ok_or_else(|| anyhow!("room id missing"))?;
        let now = Utc::now();
        let expires_at = self.base_service.expiration_for(room, self.refresh_ttl)?;
        let claims = RoomTokenClaims::refresh_token_builder(room_id, room.slug.clone())
            .role(role_key)
            .max_size(room.max_size)
            .exp(expires_at.timestamp())
            .iat(now.timestamp())
            .jti(Uuid::new_v4().to_string())
            .build_refresh_token();
        let signed_token = self.base_service.encode_claims(&claims)?;
        let record = RoomRefreshToken::new(
            room_id,
            access_jti.into(),
            &signed_token,
            expires_at.naive_utc(),
        );

        Ok(PreparedRefreshToken {
            signed_token,
            expires_at: expires_at.naive_utc(),
            record,
        })
    }

    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<RefreshTokenResponse> {
        let claims = self.base_service.decode(refresh_token)?;
        if !claims.is_refresh_token() {
            return Err(anyhow!("invalid token type, expected refresh token"));
        }
        if self
            .blacklist_repository
            .is_blacklisted(&claims.jti)
            .await?
        {
            return Err(anyhow!("refresh token is blacklisted"));
        }

        let token_hash = RoomRefreshToken::hash_token(refresh_token);
        let stored = self
            .refresh_token_repository
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or_else(|| anyhow!("refresh token not found"))?;
        if !stored.is_valid() || stored.room_id != claims.room_id {
            return Err(anyhow!("refresh token is invalid or expired"));
        }
        let stored_id = stored
            .id
            .ok_or_else(|| anyhow!("refresh token id missing"))?;

        let room = self
            .room_repository
            .find_by_id(stored.room_id)
            .await?
            .ok_or_else(|| anyhow!("room not found"))?;
        ensure_room_open(&room)?;

        // 角色归属随 refresh 旋转刷新：以 DB room_tokens 行为准，claims.role 仅兜底。
        let role_key = self
            .resolve_role_key(&stored.access_token_jti, &claims.role, &room)
            .await?;
        let (access_token, access_claims) = self.base_service.issue(&room, &role_key)?;
        let access_record = RoomToken::new(
            stored.room_id,
            access_claims.jti.clone(),
            &role_key,
            access_claims.expires_at(),
        );
        let prepared_refresh = if self.enable_rotation {
            Some(self.prepare_refresh_token(&room, &role_key, access_claims.jti.clone())?)
        } else {
            None
        };

        let rotated = self
            .access_repository
            .rotate_refresh_session(
                stored.room_id,
                &stored.access_token_jti,
                stored_id,
                &access_record,
                prepared_refresh.as_ref().map(|prepared| &prepared.record),
                Utc::now().naive_utc(),
            )
            .await?;
        if !rotated {
            return Err(anyhow!("room cannot be entered"));
        }

        if self.enable_rotation {
            let old_refresh_expires_at = claims.expires_at();
            let blacklist_entry =
                crate::models::TokenBlacklistEntry::new(claims.jti.clone(), old_refresh_expires_at);
            if let Err(error) = self.blacklist_repository.add(&blacklist_entry).await {
                log::warn!("Failed to blacklist rotated refresh token: {error}");
            }
        }

        let (next_refresh_token, next_refresh_expires_at) = match prepared_refresh {
            Some(prepared) => (prepared.signed_token, prepared.expires_at),
            None => (refresh_token.to_string(), stored.expires_at),
        };
        let capabilities = self.capabilities_for(&room, &role_key).await?;

        Ok(RefreshTokenResponse {
            access_token,
            refresh_token: next_refresh_token,
            access_token_expires_at: access_claims.expires_at(),
            refresh_token_expires_at: next_refresh_expires_at,
            role: role_key,
            capabilities,
        })
    }

    /// 解析刷新后的角色：DB 记录 > 旧 claims 快照 > 房间默认角色。
    async fn resolve_role_key(
        &self,
        access_jti: &str,
        claims_role: &str,
        room: &Room,
    ) -> Result<String> {
        if let Some(role) = self
            .access_token_repository
            .find_by_jti(access_jti)
            .await?
            .and_then(|record| record.role_key)
        {
            return Ok(role);
        }
        if !claims_role.is_empty() {
            return Ok(claims_role.to_string());
        }
        Ok(room.default_role_key.clone())
    }

    /// 从角色矩阵读取能力快照（签发响应回显；判定永远实时）。
    async fn capabilities_for(&self, room: &Room, role_key: &str) -> Result<Vec<Grant>> {
        let room_id = room.id.ok_or_else(|| anyhow!("room id missing"))?;
        let table = load_role_table(
            &self.roles_cache,
            &self.db_pool,
            room_id,
            room.roles_version,
        )
        .await?;
        Ok(table.grants(role_key).unwrap_or_default().to_vec())
    }

    pub async fn revoke_token(&self, jti: &str) -> Result<()> {
        self.access_token_repository.revoke(jti).await?;
        self.refresh_token_repository
            .revoke_by_access_jti(jti)
            .await?;
        Ok(())
    }

    pub async fn cleanup_expired(&self) -> Result<u64> {
        let deleted_refresh = self.refresh_token_repository.delete_expired().await?;
        let deleted_blacklist = self.blacklist_repository.remove_expired().await?;
        Ok(deleted_refresh + deleted_blacklist)
    }

    pub async fn verify_refresh_token(&self, token: &str) -> Result<RoomTokenClaims> {
        let claims = self.base_service.decode(token)?;
        if !claims.is_refresh_token() {
            return Err(anyhow!("invalid token type, expected refresh token"));
        }
        if self
            .blacklist_repository
            .is_blacklisted(&claims.jti)
            .await?
        {
            return Err(anyhow!("refresh token is blacklisted"));
        }
        let token_hash = RoomRefreshToken::hash_token(token);
        let stored = self
            .refresh_token_repository
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or_else(|| anyhow!("refresh token not found"))?;
        if !stored.is_valid() || stored.room_id != claims.room_id {
            return Err(anyhow!("refresh token is invalid or expired"));
        }
        let room = self
            .room_repository
            .find_by_id(stored.room_id)
            .await?
            .ok_or_else(|| anyhow!("room not found"))?;
        ensure_room_open(&room)?;
        Ok(claims)
    }

    pub fn get_access_token_ttl(&self) -> Duration {
        self.base_service.get_ttl()
    }

    pub fn get_refresh_token_ttl(&self) -> Duration {
        self.refresh_ttl
    }
}

fn ensure_room_open(room: &Room) -> Result<()> {
    if room.is_expired() {
        return Err(anyhow!("room already expired"));
    }
    if room.status() != RoomStatus::Open {
        return Err(anyhow!("room is not open"));
    }
    Ok(())
}
