use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::permission::RoomPermission;

/// 令牌类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub enum TokenType {
    /// 访问令牌（短期有效）
    #[default]
    Access,
    /// 刷新令牌（长期有效）
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RoomTokenClaims {
    pub sub: String,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub room_id: i64,
    pub room_name: String,
    pub permission: u8,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub max_size: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub exp: i64,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub iat: i64,
    pub jti: String,
    /// 令牌类型
    #[serde(default)]
    pub token_type: TokenType,
    /// 关联的刷新令牌 JTI（仅访问令牌包含此字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript-export", ts(optional))]
    pub refresh_jti: Option<String>,
}

/// 令牌构建器
#[derive(Debug, Clone)]
pub struct RoomTokenClaimsBuilder {
    room_id: i64,
    room_name: String,
    permission: u8,
    max_size: i64,
    exp: i64,
    iat: i64,
    jti: String,
    refresh_jti: Option<String>,
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

    /// 创建访问令牌构建器
    pub fn access_token_builder(room_id: i64, room_name: String) -> RoomTokenClaimsBuilder {
        let now = Utc::now();
        let jti = Uuid::new_v4().to_string();
        RoomTokenClaimsBuilder {
            room_id,
            room_name,
            permission: 0,
            max_size: 0,
            exp: now.timestamp(),
            iat: now.timestamp(),
            jti,
            refresh_jti: None,
        }
    }

    /// 创建刷新令牌构建器
    pub fn refresh_token_builder(room_id: i64, room_name: String) -> RoomTokenClaimsBuilder {
        let now = Utc::now();
        let jti = Uuid::new_v4().to_string();
        RoomTokenClaimsBuilder {
            room_id,
            room_name,
            permission: 0,
            max_size: 0,
            exp: now.timestamp(),
            iat: now.timestamp(),
            jti,
            refresh_jti: None,
        }
    }
}

impl RoomTokenClaimsBuilder {
    /// 设置权限
    pub fn permission(mut self, permission: u8) -> Self {
        self.permission = permission;
        self
    }

    /// 设置最大大小
    pub fn max_size(mut self, max_size: i64) -> Self {
        self.max_size = max_size;
        self
    }

    /// 设置过期时间
    pub fn exp(mut self, exp: i64) -> Self {
        self.exp = exp;
        self
    }

    /// 设置签发时间
    pub fn iat(mut self, iat: i64) -> Self {
        self.iat = iat;
        self
    }

    /// 设置 JTI
    pub fn jti(mut self, jti: String) -> Self {
        self.jti = jti;
        self
    }

    /// 设置关联的刷新令牌 JTI
    pub fn refresh_jti(mut self, refresh_jti: Option<String>) -> Self {
        self.refresh_jti = refresh_jti;
        self
    }

    /// 构建访问令牌
    pub fn build_access_token(self) -> RoomTokenClaims {
        RoomTokenClaims {
            sub: format!("room:{}", self.room_id),
            room_id: self.room_id,
            room_name: self.room_name,
            permission: self.permission,
            max_size: self.max_size,
            exp: self.exp,
            iat: self.iat,
            jti: self.jti,
            token_type: TokenType::Access,
            refresh_jti: self.refresh_jti,
        }
    }

    /// 构建刷新令牌
    pub fn build_refresh_token(self) -> RoomTokenClaims {
        RoomTokenClaims {
            sub: format!("room:{}", self.room_id),
            room_id: self.room_id,
            room_name: self.room_name,
            permission: self.permission,
            max_size: self.max_size,
            exp: self.exp,
            iat: self.iat,
            jti: self.jti,
            token_type: TokenType::Refresh,
            refresh_jti: None,
        }
    }
}

