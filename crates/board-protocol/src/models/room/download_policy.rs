use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, any::AnyRow, postgres::PgRow, sqlite::SqliteRow};
use utoipa::ToSchema;

use crate::models::room::row_utils::{read_datetime_from_any, read_optional_datetime_from_any};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default, sqlx::Type)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT")]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub enum DownloadPolicyMode {
    #[default]
    Off,
    Reusable,
    OneTime,
}

impl From<String> for DownloadPolicyMode {
    fn from(s: String) -> Self {
        match s.as_str() {
            "reusable" => DownloadPolicyMode::Reusable,
            "one_time" => DownloadPolicyMode::OneTime,
            _ => DownloadPolicyMode::Off,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
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

fn build_policy_from_sqlite(row: &SqliteRow) -> Result<FileDownloadPolicy, sqlx::Error> {
    Ok(FileDownloadPolicy {
        id: row.try_get("id")?,
        content_id: row.try_get("content_id")?,
        mode: row.try_get("mode")?,
        max_downloads: row.try_get("max_downloads")?,
        download_count: row.try_get("download_count")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn build_policy_from_pg(row: &PgRow) -> Result<FileDownloadPolicy, sqlx::Error> {
    Ok(FileDownloadPolicy {
        id: row.try_get("id")?,
        content_id: row.try_get("content_id")?,
        mode: row.try_get::<String, _>("mode")?.into(),
        max_downloads: row.try_get("max_downloads")?,
        download_count: row.try_get("download_count")?,
        created_at: row.try_get::<String, _>("created_at")?.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        updated_at: row.try_get::<String, _>("updated_at")?.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
    })
}

fn build_policy_from_any(row: &AnyRow) -> Result<FileDownloadPolicy, sqlx::Error> {
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

impl<'r> FromRow<'r, SqliteRow> for FileDownloadPolicy {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_policy_from_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for FileDownloadPolicy {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        // PG timestamps might be parsed as strings due to 003 migration
        let id: i64 = row.try_get("id")?;
        let content_id: i64 = row.try_get("content_id")?;
        let mode_str: String = row.try_get("mode")?;
        let max_downloads: Option<i64> = row.try_get("max_downloads")?;
        let download_count: i64 = row.try_get("download_count")?;
        
        // Custom parsing for created_at, updated_at
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;
        
        let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S%.f")
            .unwrap_or_else(|_| chrono::Utc::now().naive_utc());
        let updated_at = NaiveDateTime::parse_from_str(&updated_at_str, "%Y-%m-%d %H:%M:%S%.f")
            .unwrap_or_else(|_| chrono::Utc::now().naive_utc());

        Ok(FileDownloadPolicy {
            id: Some(id),
            content_id,
            mode: mode_str.into(),
            max_downloads,
            download_count,
            created_at,
            updated_at,
        })
    }
}

impl<'r> FromRow<'r, AnyRow> for FileDownloadPolicy {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_policy_from_any(row)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
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

fn build_code_from_sqlite(row: &SqliteRow) -> Result<FileAccessCode, sqlx::Error> {
    Ok(FileAccessCode {
        id: row.try_get("id")?,
        policy_id: row.try_get("policy_id")?,
        code_hash: row.try_get("code_hash")?,
        is_reusable: row.try_get("is_reusable")?,
        used_at: row.try_get("used_at")?,
        created_at: row.try_get("created_at")?,
    })
}

fn build_code_from_pg(row: &PgRow) -> Result<FileAccessCode, sqlx::Error> {
    let id: i64 = row.try_get("id")?;
    let policy_id: i64 = row.try_get("policy_id")?;
    let code_hash: String = row.try_get("code_hash")?;
    let is_reusable: bool = row.try_get("is_reusable")?;
    
    let created_at_str: String = row.try_get("created_at")?;
    let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S%.f")
        .unwrap_or_else(|_| chrono::Utc::now().naive_utc());
        
    let used_at = if let Ok(Some(used_str)) = row.try_get::<Option<String>, _>("used_at") {
        NaiveDateTime::parse_from_str(&used_str, "%Y-%m-%d %H:%M:%S%.f").ok()
    } else {
        None
    };

    Ok(FileAccessCode {
        id: Some(id),
        policy_id,
        code_hash,
        is_reusable,
        used_at,
        created_at,
    })
}

fn build_code_from_any(row: &AnyRow) -> Result<FileAccessCode, sqlx::Error> {
    Ok(FileAccessCode {
        id: row.try_get("id")?,
        policy_id: row.try_get("policy_id")?,
        code_hash: row.try_get("code_hash")?,
        is_reusable: row.try_get("is_reusable")?,
        used_at: read_optional_datetime_from_any(row, "used_at")?,
        created_at: read_datetime_from_any(row, "created_at")?,
    })
}

impl<'r> FromRow<'r, SqliteRow> for FileAccessCode {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        build_code_from_sqlite(row)
    }
}

impl<'r> FromRow<'r, PgRow> for FileAccessCode {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        build_code_from_pg(row)
    }
}

impl<'r> FromRow<'r, AnyRow> for FileAccessCode {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        build_code_from_any(row)
    }
}
