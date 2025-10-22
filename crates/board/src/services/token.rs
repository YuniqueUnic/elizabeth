use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::db::DbPool;
use crate::models::permission::RoomPermission;
use crate::models::{RefreshTokenResponse, Room};
use crate::repository::room_refresh_token_repository::{
    IRoomRefreshTokenRepository, ITokenBlacklistRepository,
};

const DEFAULT_LEEWAY_SECONDS: i64 = 5;
const DEFAULT_TOKEN_TTL_MINUTES: i64 = 30;
const DEFAULT_REFRESH_TOKEN_TTL_DAYS: i64 = 7;
const MINIMUM_EXP_DELTA_SECONDS: i64 = 5;

/// 令牌类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TokenType {
    /// 访问令牌（短期有效）
    #[default]
    Access,
    /// 刷新令牌（长期有效）
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomTokenClaims {
    pub sub: String,
    pub room_id: i64,
    pub room_name: String,
    pub permission: u8,
    pub max_size: i64,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    /// 令牌类型
    #[serde(default)]
    pub token_type: TokenType,
    /// 关联的刷新令牌 JTI（仅访问令牌包含此字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_jti: Option<String>,
}

impl RoomTokenClaims {
    pub fn as_permission(&self) -> RoomPermission {
        RoomPermission::from_bits(self.permission).unwrap_or_default()
    }

    pub fn expires_at(&self) -> NaiveDateTime {
        DateTime::from_timestamp(self.exp, 0)
            .unwrap_or_else(Utc::now)
            .naive_utc()
    }

    /// 检查是否为访问令牌
    pub fn is_access_token(&self) -> bool {
        matches!(self.token_type, TokenType::Access)
    }

    /// 检查是否为刷新令牌
    pub fn is_refresh_token(&self) -> bool {
        matches!(self.token_type, TokenType::Refresh)
    }

    /// 检查令牌是否已过期
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// 检查令牌是否即将过期（5 分钟内）
    pub fn is_expiring_soon(&self) -> bool {
        let five_minutes_from_now = Utc::now().timestamp() + 300; // 5 分钟 = 300 秒
        self.exp <= five_minutes_from_now
    }

    /// 获取令牌剩余有效时间（秒）
    pub fn remaining_seconds(&self) -> i64 {
        let now = Utc::now().timestamp();
        if self.exp > now { self.exp - now } else { 0 }
    }

    /// 获取令牌年龄（秒）
    pub fn age_seconds(&self) -> i64 {
        let now = Utc::now().timestamp();
        now - self.iat
    }

    #[allow(clippy::too_many_arguments)]
    /// 创建访问令牌声明
    pub fn new_access_token(
        room_id: i64,
        room_name: String,
        permission: u8,
        max_size: i64,
        exp: i64,
        iat: i64,
        jti: String,
        refresh_jti: Option<String>,
    ) -> Self {
        Self {
            sub: format!("room:{}", room_id),
            room_id,
            room_name,
            permission,
            max_size,
            exp,
            iat,
            jti,
            token_type: TokenType::Access,
            refresh_jti,
        }
    }

    /// 创建刷新令牌声明
    pub fn new_refresh_token(
        room_id: i64,
        room_name: String,
        permission: u8,
        max_size: i64,
        exp: i64,
        iat: i64,
        jti: String,
    ) -> Self {
        Self {
            sub: format!("room:{}", room_id),
            room_id,
            room_name,
            permission,
            max_size,
            exp,
            iat,
            jti,
            token_type: TokenType::Refresh,
            refresh_jti: None,
        }
    }
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

        let jti = Uuid::new_v4().to_string();
        let claims = RoomTokenClaims::new_access_token(
            room.id.ok_or_else(|| anyhow!("room id missing"))?,
            room.slug.clone(),
            room.permission.bits(),
            room.max_size,
            exp.timestamp(),
            now.timestamp(),
            jti.clone(),
            None, // 初始签发的访问令牌没有关联的刷新令牌
        );

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
