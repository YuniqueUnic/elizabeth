use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::models::{RefreshTokenResponse, Room, RoomRefreshToken, RoomStatus, RoomToken};
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
        access_jti: impl Into<String>,
    ) -> Result<PreparedRefreshToken> {
        ensure_room_open(room)?;
        let room_id = room.id.ok_or_else(|| anyhow!("room id missing"))?;
        let now = Utc::now();
        let expires_at = self.base_service.expiration_for(room, self.refresh_ttl)?;
        let claims = RoomTokenClaims::refresh_token_builder(room_id, room.slug.clone())
            .permission(room.permission.bits())
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

        let (access_token, access_claims) = self.base_service.issue(&room)?;
        let access_record = RoomToken::new(
            stored.room_id,
            access_claims.jti.clone(),
            access_claims.expires_at(),
        );
        let prepared_refresh = if self.enable_rotation {
            Some(self.prepare_refresh_token(&room, access_claims.jti.clone())?)
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

        Ok(RefreshTokenResponse {
            access_token,
            refresh_token: next_refresh_token,
            access_token_expires_at: access_claims.expires_at(),
            refresh_token_expires_at: next_refresh_expires_at,
        })
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
