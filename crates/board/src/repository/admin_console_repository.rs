//! 平台管理后台（issue #196）的只读统计与房间管理查询。
//!
//! 面板是报表型只读界面，聚合查询集中在此；写路径复用既有
//! RoomRepository / RoomLifecycleService，不另起第二套写模型。

use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;

use crate::db::DbPool;
use crate::models::room::row_utils::{read_datetime_from_any, read_optional_datetime_from_any};

pub struct AdminConsoleRepository {
    pool: Arc<DbPool>,
}

impl AdminConsoleRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    /// Dashboard 统计的存储与内容聚合。
    pub async fn content_stats(&self) -> Result<(i64, i64, i64, i64, i64, i64)> {
        let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM room_contents),
                (SELECT COUNT(*) FROM room_contents WHERE content_type = 2),
                (SELECT COUNT(*) FROM room_contents WHERE content_type = 0),
                (SELECT COALESCE(SUM(size), 0) FROM room_contents),
                (SELECT COALESCE(SUM(size), 0) FROM room_content_blobs),
                (SELECT COUNT(*) FROM room_content_blobs)
            "#,
        )
        .fetch_one(self.pool.as_ref())
        .await
        .context("failed to aggregate content stats")?;
        Ok(row)
    }

    /// 平台房间统计。
    pub async fn room_stats(&self) -> Result<(i64, i64, i64)> {
        let row = sqlx::query_as::<_, (i64, i64, i64)>(
            r#"
            SELECT
                COUNT(*),
                COALESCE(SUM(CASE WHEN status = 0 AND (expire_at IS NULL OR expire_at > $1) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN password IS NOT NULL THEN 1 ELSE 0 END), 0)
            FROM rooms
            "#,
        )
        .bind(chrono::Utc::now().naive_utc().to_string())
        .fetch_one(self.pool.as_ref())
        .await
        .context("failed to aggregate room stats")?;
        Ok(row)
    }

    /// 房间管理列表：关键字搜索（name/slug）+ 分页。
    pub async fn list_rooms(
        &self,
        query: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AdminRoomRow>> {
        let pattern = query.map(|q| format!("%{}%", q));
        let rows = sqlx::query(
            r#"
            SELECT
                r.id, r.name, r.slug, r.status,
                CASE WHEN r.password IS NOT NULL THEN 1 ELSE 0 END AS password_protected,
                r.current_size, r.max_size,
                r.current_times_entered, r.max_times_entered,
                CAST(r.expire_at AS TEXT) AS expire_at,
                CAST(r.created_at AS TEXT) AS created_at,
                CAST(r.updated_at AS TEXT) AS updated_at,
                (SELECT COUNT(*) FROM room_contents c WHERE c.room_id = r.id) AS content_count
            FROM rooms r
            WHERE r.name LIKE COALESCE($1, '%') OR r.slug LIKE COALESCE($1, '%')
            ORDER BY r.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.as_ref())
        .await
        .context("failed to list rooms for admin")?
        .iter()
        .map(parse_admin_room_row)
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .context("failed to decode admin room rows")?;
        Ok(rows)
    }

    pub async fn count_rooms(&self, query: Option<&str>) -> Result<i64> {
        let pattern = query.map(|q| format!("%{}%", q));
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rooms WHERE name LIKE COALESCE($1, '%') OR slug LIKE COALESCE($1, '%')",
        )
        .bind(pattern)
        .fetch_one(self.pool.as_ref())
        .await
        .context("failed to count rooms for admin")?;
        Ok(total)
    }

    /// 房间详情的附属计数（内容寻址 blob / 会话 token）。
    pub async fn room_detail_counts(&self, room_id: i64) -> Result<(i64, i64)> {
        let row = sqlx::query_as::<_, (i64, i64)>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM room_content_blobs WHERE room_id = $1),
                (SELECT COUNT(*) FROM room_tokens
                 WHERE room_id = $1 AND revoked_at IS NULL AND expires_at > $2)
            "#,
        )
        .bind(room_id)
        .bind(chrono::Utc::now().naive_utc().to_string())
        .fetch_one(self.pool.as_ref())
        .await
        .context("failed to aggregate room detail counts")?;
        Ok(row)
    }
}

/// 管理列表行；时间列统一以文本读出后交给 Any 驱动解码。
pub struct AdminRoomRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub status: crate::models::RoomStatus,
    pub password_protected: bool,
    pub current_size: i64,
    pub max_size: i64,
    pub current_times_entered: i64,
    pub max_times_entered: i64,
    pub expire_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub content_count: i64,
}

fn parse_admin_room_row(row: &sqlx::any::AnyRow) -> Result<AdminRoomRow, sqlx::Error> {
    use sqlx::Row;
    Ok(AdminRoomRow {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        slug: row.try_get("slug")?,
        status: row.try_get("status")?,
        password_protected: row.try_get::<i64, _>("password_protected")? != 0,
        current_size: row.try_get("current_size")?,
        max_size: row.try_get("max_size")?,
        current_times_entered: row.try_get("current_times_entered")?,
        max_times_entered: row.try_get("max_times_entered")?,
        expire_at: read_optional_datetime_from_any(row, "expire_at")?,
        created_at: read_datetime_from_any(row, "created_at")?,
        updated_at: read_datetime_from_any(row, "updated_at")?,
        content_count: row.try_get("content_count")?,
    })
}
