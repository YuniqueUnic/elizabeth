//! 平台管理员账号与管理 API key 的存储模型。
//!
//! 两者只服务于服务端鉴权，不直接暴露为 API schema；
//! 对外视图见 [`crate::dto::admin`]。

use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow};

use crate::models::room::row_utils::{read_datetime_from_any, read_optional_datetime_from_any};

/// 平台管理员账号；密码以 Argon2 PHC 字符串存储。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAccount {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl AdminAccount {
    /// 密码版本号：改密即更新 `updated_at`，使旧会话令牌（记录签发时的版本）全部失效。
    pub fn password_version(&self) -> i64 {
        self.updated_at.and_utc().timestamp()
    }
}

/// 平台管理 API key；明文只在创建时返回一次，库中仅存 SHA-256 与可定位前缀。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminApiKey {
    pub id: i64,
    pub name: String,
    pub prefix: String,
    pub key_hash: String,
    pub expires_at: Option<NaiveDateTime>,
    pub revoked_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub last_used_at: Option<NaiveDateTime>,
}

impl AdminApiKey {
    pub fn is_active_at(&self, now: NaiveDateTime) -> bool {
        self.revoked_at.is_none() && self.expires_at.is_none_or(|expires_at| expires_at > now)
    }

    pub fn is_active(&self) -> bool {
        self.is_active_at(Utc::now().naive_utc())
    }
}

impl<'r> FromRow<'r, AnyRow> for AdminAccount {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(AdminAccount {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            password_hash: row.try_get("password_hash")?,
            created_at: read_datetime_from_any(row, "created_at")?,
            updated_at: read_datetime_from_any(row, "updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, AnyRow> for AdminApiKey {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(AdminApiKey {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            prefix: row.try_get("prefix")?,
            key_hash: row.try_get("key_hash")?,
            expires_at: read_optional_datetime_from_any(row, "expires_at")?,
            revoked_at: read_optional_datetime_from_any(row, "revoked_at")?,
            created_at: read_datetime_from_any(row, "created_at")?,
            last_used_at: read_optional_datetime_from_any(row, "last_used_at")?,
        })
    }
}
