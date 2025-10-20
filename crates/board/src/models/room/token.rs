use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomToken {
    pub id: Option<i64>,
    pub room_id: i64,
    pub jti: String,
    pub expires_at: NaiveDateTime,
    pub revoked_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
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
