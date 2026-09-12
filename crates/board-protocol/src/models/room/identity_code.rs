use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow, postgres::PgRow, sqlite::SqliteRow};
use utoipa::ToSchema;

use crate::models::room::row_utils::{read_datetime_from_any, read_optional_datetime_from_any};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomIdentityCode {
    pub id: Option<i64>,
    pub room_id: i64,
    /// Only used for server-side verification and never returned by the API.
    pub code_hash: String,
    pub role_key: String,
    pub expires_at: NaiveDateTime,
    pub revoked_at: Option<NaiveDateTime>,
    pub created_by_jti: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

fn build_sqlite(row: &SqliteRow) -> Result<RoomIdentityCode, sqlx::Error> {
    Ok(RoomIdentityCode {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        code_hash: row.try_get("code_hash")?,
        role_key: row.try_get("role_key")?,
        expires_at: row.try_get("expires_at")?,
        revoked_at: row.try_get("revoked_at")?,
        created_by_jti: row.try_get("created_by_jti")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn build_pg(row: &PgRow) -> Result<RoomIdentityCode, sqlx::Error> {
    Ok(RoomIdentityCode {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        code_hash: row.try_get("code_hash")?,
        role_key: row.try_get("role_key")?,
        expires_at: row.try_get("expires_at")?,
        revoked_at: row.try_get("revoked_at")?,
        created_by_jti: row.try_get("created_by_jti")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn build_any(row: &AnyRow) -> Result<RoomIdentityCode, sqlx::Error> {
    Ok(RoomIdentityCode {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        code_hash: row.try_get("code_hash")?,
        role_key: row.try_get("role_key")?,
        expires_at: read_datetime_from_any(row, "expires_at")?,
        revoked_at: read_optional_datetime_from_any(row, "revoked_at")?,
        created_by_jti: row.try_get("created_by_jti")?,
        created_at: read_datetime_from_any(row, "created_at")?,
        updated_at: read_datetime_from_any(row, "updated_at")?,
    })
}

impl<'r> FromRow<'r, SqliteRow> for RoomIdentityCode {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for RoomIdentityCode {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_pg(row)
    }
}

impl<'r> FromRow<'r, AnyRow> for RoomIdentityCode {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_any(row)
    }
}

impl RoomIdentityCode {
    pub fn is_active_at(&self, now: NaiveDateTime) -> bool {
        self.revoked_at.is_none() && self.expires_at > now
    }

    pub fn is_active(&self) -> bool {
        self.is_active_at(Utc::now().naive_utc())
    }
}
