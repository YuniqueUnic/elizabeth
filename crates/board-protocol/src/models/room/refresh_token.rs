use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Row, any::AnyRow, postgres::PgRow, sqlite::SqliteRow};
use utoipa::ToSchema;

use crate::models::room::row_utils::{read_datetime_from_any, read_optional_datetime_from_any};

/// 房间刷新令牌数据模型
/// 用于存储和管理 JWT 刷新令牌的信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RoomRefreshToken {
    /// 主键 ID
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    pub id: Option<i64>,
    /// 关联的房间 ID
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub room_id: i64,
    /// 关联的访问令牌 JTI
    pub access_token_jti: String,
    /// 刷新令牌的 SHA-256 哈希值（不存储明文）
    pub token_hash: String,
    /// 刷新令牌过期时间
    pub expires_at: NaiveDateTime,
    /// 创建时间
    pub created_at: NaiveDateTime,
    /// 最后使用时间
    pub last_used_at: Option<NaiveDateTime>,
    /// 是否已撤销
    pub is_revoked: bool,
}

fn build_room_refresh_token_sqlite(row: &SqliteRow) -> Result<RoomRefreshToken, sqlx::Error> {
    Ok(RoomRefreshToken {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        access_token_jti: row.try_get("access_token_jti")?,
        token_hash: row.try_get("token_hash")?,
        expires_at: row.try_get("expires_at")?,
        created_at: row.try_get("created_at")?,
        last_used_at: row.try_get("last_used_at")?,
        is_revoked: row.try_get("is_revoked")?,
    })
}

fn build_room_refresh_token_pg(row: &PgRow) -> Result<RoomRefreshToken, sqlx::Error> {
    Ok(RoomRefreshToken {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        access_token_jti: row.try_get("access_token_jti")?,
        token_hash: row.try_get("token_hash")?,
        expires_at: row.try_get("expires_at")?,
        created_at: row.try_get("created_at")?,
        last_used_at: row.try_get("last_used_at")?,
        is_revoked: row.try_get("is_revoked")?,
    })
}

fn build_room_refresh_token_any(row: &AnyRow) -> Result<RoomRefreshToken, sqlx::Error> {
    let is_revoked_raw: i64 = row.try_get("is_revoked")?;
    Ok(RoomRefreshToken {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        access_token_jti: row.try_get("access_token_jti")?,
        token_hash: row.try_get("token_hash")?,
        expires_at: read_datetime_from_any(row, "expires_at")?,
        created_at: read_datetime_from_any(row, "created_at")?,
        last_used_at: read_optional_datetime_from_any(row, "last_used_at")?,
        is_revoked: is_revoked_raw != 0,
    })
}

impl<'r> FromRow<'r, SqliteRow> for RoomRefreshToken {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_room_refresh_token_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for RoomRefreshToken {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_room_refresh_token_pg(row)
    }
}

impl<'r> FromRow<'r, AnyRow> for RoomRefreshToken {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_room_refresh_token_any(row)
    }
}

impl RoomRefreshToken {
    /// 创建新的刷新令牌记录
    pub fn new(
        room_id: i64,
        access_token_jti: String,
        refresh_token: &str,
        expires_at: NaiveDateTime,
    ) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id: None,
            room_id,
            access_token_jti,
            token_hash: Self::hash_token(refresh_token),
            expires_at,
            created_at: now,
            last_used_at: None,
            is_revoked: false,
        }
    }

    /// 对刷新令牌进行 SHA-256 哈希处理
    pub fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 验证刷新令牌是否匹配存储的哈希值
    pub fn verify_token(&self, token: &str) -> bool {
        self.token_hash == Self::hash_token(token)
    }

    /// 检查刷新令牌是否已过期
    pub fn is_expired(&self) -> bool {
        Utc::now().naive_utc() > self.expires_at
    }

    /// 检查刷新令牌是否有效（未过期且未撤销）
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_revoked
    }

    /// 检查刷新令牌是否即将过期（1 小时内）
    pub fn is_expiring_soon(&self) -> bool {
        let one_hour_from_now = Utc::now().naive_utc() + chrono::Duration::hours(1);
        self.expires_at <= one_hour_from_now
    }

    /// 更新最后使用时间
    pub fn update_last_used(&mut self) {
        self.last_used_at = Some(Utc::now().naive_utc());
    }

    /// 撤销刷新令牌
    pub fn revoke(&mut self) {
        self.is_revoked = true;
    }

    /// 获取令牌剩余有效时间（秒）
    pub fn remaining_seconds(&self) -> i64 {
        let now = Utc::now().naive_utc();
        if self.expires_at > now {
            (self.expires_at - now).num_seconds()
        } else {
            0
        }
    }

    /// 获取令牌年龄（秒）
    pub fn age_seconds(&self) -> i64 {
        let now = Utc::now().naive_utc();
        (now - self.created_at).num_seconds()
    }
}

/// 用于创建刷新令牌的请求结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct CreateRefreshTokenRequest {
    /// 房间 ID
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub room_id: i64,
    /// 访问令牌 JTI
    pub access_token_jti: String,
    /// 刷新令牌（明文，仅用于创建时哈希）
    pub refresh_token: String,
    /// 过期时间
    pub expires_at: NaiveDateTime,
}

/// 刷新令牌验证请求结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RefreshTokenRequest {
    /// 刷新令牌（明文）
    pub refresh_token: String,
}

/// 刷新令牌验证响应结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct RefreshTokenResponse {
    /// 新的访问令牌
    pub access_token: String,
    /// 新的刷新令牌
    pub refresh_token: String,
    /// 访问令牌过期时间
    pub access_token_expires_at: NaiveDateTime,
    /// 刷新令牌过期时间
    pub refresh_token_expires_at: NaiveDateTime,
}

/// 令牌黑名单条目结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct TokenBlacklistEntry {
    /// 主键 ID
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    pub id: Option<i64>,
    /// 令牌 JTI
    pub jti: String,
    /// 过期时间（黑名单记录的过期时间）
    pub expires_at: NaiveDateTime,
    /// 创建时间
    pub created_at: NaiveDateTime,
}

fn build_token_blacklist_entry_sqlite(row: &SqliteRow) -> Result<TokenBlacklistEntry, sqlx::Error> {
    Ok(TokenBlacklistEntry {
        id: row.try_get("id")?,
        jti: row.try_get("jti")?,
        expires_at: row.try_get("expires_at")?,
        created_at: row.try_get("created_at")?,
    })
}

fn build_token_blacklist_entry_pg(row: &PgRow) -> Result<TokenBlacklistEntry, sqlx::Error> {
    Ok(TokenBlacklistEntry {
        id: row.try_get("id")?,
        jti: row.try_get("jti")?,
        expires_at: row.try_get("expires_at")?,
        created_at: row.try_get("created_at")?,
    })
}

fn build_token_blacklist_entry_any(row: &AnyRow) -> Result<TokenBlacklistEntry, sqlx::Error> {
    Ok(TokenBlacklistEntry {
        id: row.try_get("id")?,
        jti: row.try_get("jti")?,
        expires_at: read_datetime_from_any(row, "expires_at")?,
        created_at: read_datetime_from_any(row, "created_at")?,
    })
}

impl<'r> FromRow<'r, SqliteRow> for TokenBlacklistEntry {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_token_blacklist_entry_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for TokenBlacklistEntry {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_token_blacklist_entry_pg(row)
    }
}

impl<'r> FromRow<'r, AnyRow> for TokenBlacklistEntry {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_token_blacklist_entry_any(row)
    }
}

impl TokenBlacklistEntry {
    /// 创建新的黑名单条目
    pub fn new(jti: String, expires_at: NaiveDateTime) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id: None,
            jti,
            expires_at,
            created_at: now,
        }
    }

    /// 检查黑名单条目是否已过期
    pub fn is_expired(&self) -> bool {
        Utc::now().naive_utc() > self.expires_at
    }

    /// 检查黑名单条目是否有效
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }
}
