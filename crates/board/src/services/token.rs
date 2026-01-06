use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use chrono::{Duration, NaiveDateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};

use crate::db::DbPool;
use crate::models::{RefreshTokenResponse, Room};
use crate::repository::room_refresh_token_repository::{
    IRoomRefreshTokenRepository, ITokenBlacklistRepository,
};

pub use crate::dto::{RoomTokenClaims, RoomTokenClaimsBuilder, TokenType};

const DEFAULT_LEEWAY_SECONDS: i64 = 5;
const DEFAULT_TOKEN_TTL_MINUTES: i64 = 120;
const DEFAULT_REFRESH_TOKEN_TTL_DAYS: i64 = 7;
const MINIMUM_EXP_DELTA_SECONDS: i64 = 5;

#[derive(Clone)]
pub struct RoomTokenService {
    secret: Arc<String>,
    ttl: Duration,
    leeway: i64,
}

impl RoomTokenService {
    pub fn new(secret: Arc<String>) -> Self {
        Self::with_options(
            secret,
            Duration::minutes(DEFAULT_TOKEN_TTL_MINUTES),
            DEFAULT_LEEWAY_SECONDS,
        )
    }

    pub fn with_ttl(secret: Arc<String>, ttl: Duration) -> Self {
        Self::with_options(secret, ttl, DEFAULT_LEEWAY_SECONDS)
    }

    pub fn with_options(secret: Arc<String>, ttl: Duration, leeway_seconds: i64) -> Self {
        let ttl = if ttl.num_seconds() < MINIMUM_EXP_DELTA_SECONDS {
            Duration::seconds(MINIMUM_EXP_DELTA_SECONDS + 1)
        } else {
            ttl
        };
        Self {
            secret,
            ttl,
            leeway: leeway_seconds.max(0),
        }
    }

    pub fn with_config(secret: Arc<String>, ttl_seconds: i64, leeway_seconds: i64) -> Self {
        let ttl_seconds = ttl_seconds.max(MINIMUM_EXP_DELTA_SECONDS + 1);
        Self::with_options(secret, Duration::seconds(ttl_seconds), leeway_seconds)
    }

    pub fn issue(&self, room: &Room) -> Result<(String, RoomTokenClaims)> {
        if room.is_expired() {
            return Err(anyhow!("room already expired"));
        }

        let now = Utc::now();
        let mut exp = now + self.ttl;

        if let Some(room_expire) = room.expire_at {
            let room_expire = room_expire - chrono::Duration::seconds(self.leeway);
            if room_expire <= now.naive_utc() {
                return Err(anyhow!("room expires too soon to issue token"));
            }
            let room_expire_dt =
                chrono::DateTime::<Utc>::from_naive_utc_and_offset(room_expire, Utc);
            if exp > room_expire_dt {
                exp = room_expire_dt;
            }
        }

        if (exp - now).num_seconds() < MINIMUM_EXP_DELTA_SECONDS {
            return Err(anyhow!(
                "token ttl too short after applying room expiry limit"
            ));
        }

        let claims = RoomTokenClaims::access_token_builder(
            room.id.ok_or_else(|| anyhow!("room id missing"))?,
            room.slug.clone(),
        )
        .permission(room.permission.bits())
        .max_size(room.max_size)
        .exp(exp.timestamp())
        .iat(now.timestamp())
        .refresh_jti(None) // 初始签发的访问令牌没有关联的刷新令牌
        .build_access_token();

        let token = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .context("failed to sign room token")?;

        Ok((token, claims))
    }

    pub fn decode(&self, token: &str) -> Result<RoomTokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = self.leeway as u64;
        let data = jsonwebtoken::decode::<RoomTokenClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .context("invalid token")?;

        Ok(data.claims)
    }

    /// 编码令牌声明
    pub fn encode_claims(&self, claims: &RoomTokenClaims) -> Result<String> {
        jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .context("failed to encode token")
    }

    /// 获取密钥
    pub fn get_secret(&self) -> &Arc<String> {
        &self.secret
    }

    /// 获取 TTL
    pub fn get_ttl(&self) -> Duration {
        self.ttl
    }

    /// 获取宽限期
    pub fn get_leeway(&self) -> i64 {
        self.leeway
    }
}
