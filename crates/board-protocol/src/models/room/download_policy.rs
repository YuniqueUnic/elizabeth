use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow};
use utoipa::ToSchema;

use crate::models::room::row_utils::{
    read_bool_from_any, read_datetime_from_any, read_optional_datetime_from_any,
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default, sqlx::Type,
)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum DownloadPolicyMode {
    #[default]
    Off,
    Reusable,
    OneTime,
}

impl From<String> for DownloadPolicyMode {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "reusable" => DownloadPolicyMode::Reusable,
            "one_time" => DownloadPolicyMode::OneTime,
            _ => DownloadPolicyMode::Off,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct FileDownloadPolicy {
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    pub id: Option<i64>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub content_id: i64,
    pub mode: DownloadPolicyMode,
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    pub max_downloads: Option<i64>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub download_count: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl<'r> FromRow<'r, AnyRow> for FileDownloadPolicy {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        let mode_str: String = row.try_get("mode")?;
        Ok(FileDownloadPolicy {
            id: row.try_get("id")?,
            content_id: row.try_get("content_id")?,
            mode: mode_str.into(),
            max_downloads: row.try_get("max_downloads")?,
            download_count: row.try_get("download_count")?,
            created_at: read_datetime_from_any(row, "created_at")?,
            updated_at: read_datetime_from_any(row, "updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct FileAccessCode {
    #[cfg_attr(feature = "typescript-export", ts(type = "number | null"))]
    pub id: Option<i64>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub policy_id: i64,
    pub code_hash: String,
    pub is_reusable: bool,
    pub used_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

impl<'r> FromRow<'r, AnyRow> for FileAccessCode {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(FileAccessCode {
            id: row.try_get("id")?,
            policy_id: row.try_get("policy_id")?,
            code_hash: row.try_get("code_hash")?,
            is_reusable: read_bool_from_any(row, "is_reusable")?,
            used_at: read_optional_datetime_from_any(row, "used_at")?,
            created_at: read_datetime_from_any(row, "created_at")?,
        })
    }
}
