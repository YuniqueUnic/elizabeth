use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow, postgres::PgRow, sqlite::SqliteRow};
use utoipa::ToSchema;

use crate::models::room::row_utils::{read_datetime_from_any, read_optional_datetime_from_any};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomToken {
    pub id: Option<i64>,
    pub room_id: i64,
    pub jti: String,
    pub expires_at: NaiveDateTime,
    pub revoked_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

fn build_room_token_from_row<R, F, G>(
    row: &R,
    read_dt: F,
    read_optional_dt: G,
) -> Result<RoomToken, sqlx::Error>
where
    R: Row,
    F: Fn(&R, &str) -> Result<NaiveDateTime, sqlx::Error>,
    G: Fn(&R, &str) -> Result<Option<NaiveDateTime>, sqlx::Error>,
{
    Ok(RoomToken {
        id: row.try_get("id")?,
        room_id: row.try_get("room_id")?,
        jti: row.try_get("jti")?,
        expires_at: read_dt(row, "expires_at")?,
        revoked_at: read_optional_dt(row, "revoked_at")?,
        created_at: read_dt(row, "created_at")?,
    })
}

impl<'r> FromRow<'r, SqliteRow> for RoomToken {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_room_token_from_row(
            row,
            |row, column| row.try_get(column),
            |row, column| row.try_get(column),
        )
    }
}

impl<'r> FromRow<'r, PgRow> for RoomToken {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_room_token_from_row(
            row,
            |row, column| row.try_get(column),
            |row, column| row.try_get(column),
        )
    }
}

impl<'r> FromRow<'r, AnyRow> for RoomToken {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_room_token_from_row(row, read_datetime_from_any, read_optional_datetime_from_any)
    }
}

impl RoomToken {
    pub fn new(room_id: i64, jti: impl Into<String>, expires_at: NaiveDateTime) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id: None,
            room_id,
            jti: jti.into(),
            expires_at,
            revoked_at: None,
            created_at: now,
        }
    }

    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at > Utc::now().naive_utc()
    }
}
