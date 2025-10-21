use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::Room;
use crate::models::permission::RoomPermission;

const DEFAULT_LEEWAY_SECONDS: i64 = 5;
const DEFAULT_TOKEN_TTL_MINUTES: i64 = 30;
const MINIMUM_EXP_DELTA_SECONDS: i64 = 5;

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
}

#[derive(Clone)]
pub struct RoomTokenService {
    secret: Arc<String>,
    ttl: Duration,
    leeway: i64,
}

impl RoomTokenService {
    pub fn new(secret: Arc<String>) -> Self {
        Self::with_ttl(secret, Duration::minutes(DEFAULT_TOKEN_TTL_MINUTES))
    }

    pub fn with_ttl(secret: Arc<String>, ttl: Duration) -> Self {
        Self {
            secret,
            ttl,
            leeway: DEFAULT_LEEWAY_SECONDS,
        }
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
        let claims = RoomTokenClaims {
            sub: format!("room:{}", room.id.unwrap_or_default()),
            room_id: room.id.ok_or_else(|| anyhow!("room id missing"))?,
            room_name: room.slug.clone(),
            permission: room.permission.bits(),
            max_size: room.max_size,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti,
        };

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
}
