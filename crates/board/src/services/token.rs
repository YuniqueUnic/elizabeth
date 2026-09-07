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

/// 身份码允许配置的最短有效时长（秒）。
pub const MIN_IDENTITY_TTL_SECONDS: i64 = 60;
/// 身份码允许配置的最长有效时长（10 年）；实际有效期仍会被房间过期时间封顶。
pub const MAX_IDENTITY_TTL_SECONDS: i64 = 31_536_000 * 10;

/// "跟随房间生命周期"的请求 TTL：远超任何房间有效期，
/// 由 `expiration_for` 统一封顶到房间过期时刻。
pub fn room_lifetime_ttl() -> Duration {
    Duration::days(365 * 100)
}

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

    pub fn issue(&self, room: &Room, role_key: &str) -> Result<(String, RoomTokenClaims)> {
        self.issue_with_ttl(room, role_key, self.ttl)
    }

    /// 以指定有效时长签发访问令牌；实际过期时间不会超过房间自身的过期时间。
    pub fn issue_with_ttl(
        &self,
        room: &Room,
        role_key: &str,
        requested_ttl: Duration,
    ) -> Result<(String, RoomTokenClaims)> {
        if room.is_expired() {
            return Err(anyhow!("room already expired"));
        }

        let now = Utc::now();
        let exp = self.expiration_for(room, requested_ttl)?;

        let claims = RoomTokenClaims::access_token_builder(
            room.id.ok_or_else(|| anyhow!("room id missing"))?,
            room.slug.clone(),
        )
        .role(role_key)
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

    pub(crate) fn expiration_for(
        &self,
        room: &Room,
        requested_ttl: Duration,
    ) -> Result<chrono::DateTime<Utc>> {
        let now = Utc::now();
        let mut expiration = now + requested_ttl;

        if let Some(room_expire) = room.expire_at {
            let room_expire = room_expire - Duration::seconds(self.leeway);
            if room_expire <= now.naive_utc() {
                return Err(anyhow!("room expires too soon to issue token"));
            }
            let room_expire = chrono::DateTime::<Utc>::from_naive_utc_and_offset(room_expire, Utc);
            expiration = expiration.min(room_expire);
        }

        if (expiration - now).num_seconds() < MINIMUM_EXP_DELTA_SECONDS {
            return Err(anyhow!(
                "token ttl too short after applying room expiry limit"
            ));
        }

        Ok(expiration)
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
