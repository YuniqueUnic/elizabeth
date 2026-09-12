//! 房间内容 blob 引用计数仓储（issue #200）。
//!
//! (room_id, hash) 唯一指向一份物理对象；内容记录通过 hash 引用 blob，
//! 引用计数归零后由调用方物理删除并清理本行。去重范围 per-room，
//! 跨房间共享去重是存在性 oracle，默认不开启（见 issue #200 / #196）。

use async_trait::async_trait;
use chrono::Utc;

use std::sync::Arc;

use crate::db::DbPool;

#[derive(Debug, Clone)]
pub struct RoomContentBlob {
    pub id: Option<i64>,
    pub room_id: i64,
    /// 内容 SHA-256（小写 hex）
    pub hash: String,
    /// 物理对象的存储 locator
    pub locator: String,
    pub size: i64,
    pub ref_count: i64,
}

#[async_trait]
pub trait IRoomContentBlobRepository: Send + Sync {
    /// 按哈希查找 blob（秒传命中判定）。
    async fn find_by_hash(
        &self,
        room_id: i64,
        hash: &str,
    ) -> anyhow::Result<Option<RoomContentBlob>>;
    /// 上传落账：不存在则建行（ref_count = 1），存在则引用 +1。原子 upsert。
    async fn upsert_ref(&self, blob: RoomContentBlob) -> anyhow::Result<RoomContentBlob>;
    /// 引用计数 -1；归零时删除本行并返回 None，由调用方物理删除对象。
    async fn decrement_ref(&self, id: i64) -> anyhow::Result<Option<RoomContentBlob>>;
    /// 删除房间全部 blob 行（房间 GC；物理对象由后端 purge_room 清理）。
    async fn delete_by_room(&self, room_id: i64) -> anyhow::Result<()>;
}

pub struct RoomContentBlobRepository {
    pool: Arc<DbPool>,
}

impl RoomContentBlobRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

fn now_str() -> String {
    Utc::now().naive_utc().to_string()
}

fn parse_blob(row: &sqlx::any::AnyRow) -> anyhow::Result<RoomContentBlob> {
    use sqlx::Row;
    Ok(RoomContentBlob {
        id: row.try_get::<Option<i64>, _>("id")?,
        room_id: row.try_get::<i64, _>("room_id")?,
        hash: row.try_get::<String, _>("hash")?,
        locator: row.try_get::<String, _>("locator")?,
        size: row.try_get::<i64, _>("size")?,
        ref_count: row.try_get::<i64, _>("ref_count")?,
    })
}

#[async_trait]
impl IRoomContentBlobRepository for RoomContentBlobRepository {
    async fn find_by_hash(
        &self,
        room_id: i64,
        hash: &str,
    ) -> anyhow::Result<Option<RoomContentBlob>> {
        let row = sqlx::query(
            r#"
            SELECT id, room_id, hash, locator, size, ref_count
            FROM room_content_blobs
            WHERE room_id = $1 AND hash = $2
            "#,
        )
        .bind(room_id)
        .bind(hash)
        .fetch_optional(self.pool.as_ref())
        .await?;
        match row {
            Some(row) => Ok(Some(parse_blob(&row)?)),
            None => Ok(None),
        }
    }

    async fn upsert_ref(&self, blob: RoomContentBlob) -> anyhow::Result<RoomContentBlob> {
        let row = sqlx::query(
            r#"
            INSERT INTO room_content_blobs
                (room_id, hash, locator, size, ref_count, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 1, $5, $6)
            ON CONFLICT(room_id, hash) DO UPDATE
                SET ref_count = room_content_blobs.ref_count + 1, updated_at = $6
            RETURNING id, room_id, hash, locator, size, ref_count
            "#,
        )
        .bind(blob.room_id)
        .bind(&blob.hash)
        .bind(&blob.locator)
        .bind(blob.size)
        .bind(now_str())
        .bind(now_str())
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(parse_blob(&row)?)
    }

    async fn decrement_ref(&self, id: i64) -> anyhow::Result<Option<RoomContentBlob>> {
        sqlx::query("UPDATE room_content_blobs SET ref_count = ref_count - 1, updated_at = $2 WHERE id = $1")
            .bind(id)
            .bind(now_str())
            .execute(self.pool.as_ref())
            .await?;
        let row = sqlx::query(
            "SELECT id, room_id, hash, locator, size, ref_count FROM room_content_blobs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool.as_ref())
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let blob = parse_blob(&row)?;
        if blob.ref_count > 0 {
            return Ok(Some(blob));
        }
        sqlx::query("DELETE FROM room_content_blobs WHERE id = $1")
            .bind(id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(None)
    }

    async fn delete_by_room(&self, room_id: i64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM room_content_blobs WHERE room_id = $1")
            .bind(room_id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }
}
