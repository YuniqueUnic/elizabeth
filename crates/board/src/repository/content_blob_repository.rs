//! 内容寻址 blob 引用计数仓储（issue #200 / #196）。
//!
//! `(hash, owner_room_id)` 唯一定位一份物理对象；`owner_room_id` 是首次存入
//! 的房间。去重作用域由配置决定：per-room（默认）查找限定 owner 房间，
//! 全局模式跨房间命中并共享引用计数——跨房间共享构成存在性 oracle，
//! 仅作为启动期显式配置开放。引用归零的物理删除由调用方执行。

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::db::DbPool;

#[derive(Debug, Clone)]
pub struct ContentBlob {
    pub hash: String,
    /// 首次存入的房间
    pub owner_room_id: i64,
    /// 物理对象的存储 locator
    pub locator: String,
    pub size: i64,
    pub ref_count: i64,
}

#[async_trait]
pub trait IContentBlobRepository: Send + Sync {
    /// 按哈希查找 blob。`scope_room_id`：
    /// - `Some(room)` — per-room 去重：仅命中 owner 为该房间的行；
    /// - `None` — 全局去重：命中任意行。
    async fn find_by_hash(
        &self,
        hash: &str,
        scope_room_id: Option<i64>,
    ) -> anyhow::Result<Option<ContentBlob>>;
    /// 上传落账：同 (hash, owner) 存在则引用 +1，否则建行（ref_count = 1）。原子 upsert。
    async fn upsert_ref(&self, blob: ContentBlob) -> anyhow::Result<ContentBlob>;
    /// 内容删除：优先递减 owner 为该房间的行，其次同哈希任意行（全局引用）。
    /// 归零时删除行并返回被清零的 blob，由调用方物理删除对象。
    async fn decrement_for(&self, room_id: i64, hash: &str) -> anyhow::Result<Option<ContentBlob>>;
    /// 房间回收：按 (hash, 引用数) 批量递减本房间贡献的引用。
    /// 返回所有被清零并删除行的 blob，由调用方物理删除对象。
    async fn release_room(
        &self,
        entries: &[(String, i64)],
        room_id: i64,
    ) -> anyhow::Result<Vec<ContentBlob>>;
    /// 房间详情计数：owner 为本房间的 blob 行数。
    async fn count_by_owner(&self, room_id: i64) -> anyhow::Result<i64>;
}

pub struct ContentBlobRepository {
    pool: Arc<DbPool>,
}

impl ContentBlobRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

fn now_str() -> String {
    Utc::now().naive_utc().to_string()
}

fn parse_blob(row: &sqlx::any::AnyRow) -> anyhow::Result<ContentBlob> {
    use sqlx::Row;
    Ok(ContentBlob {
        hash: row.try_get("hash")?,
        owner_room_id: row.try_get("owner_room_id")?,
        locator: row.try_get("locator")?,
        size: row.try_get("size")?,
        ref_count: row.try_get("ref_count")?,
    })
}

#[async_trait]
impl IContentBlobRepository for ContentBlobRepository {
    async fn find_by_hash(
        &self,
        hash: &str,
        scope_room_id: Option<i64>,
    ) -> anyhow::Result<Option<ContentBlob>> {
        let row = match scope_room_id {
            Some(room_id) => {
                sqlx::query(
                    r#"
                    SELECT hash, owner_room_id, locator, size, ref_count
                    FROM content_blobs
                    WHERE hash = $1 AND owner_room_id = $2
                    "#,
                )
                .bind(hash)
                .bind(room_id)
                .fetch_optional(self.pool.as_ref())
                .await?
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT hash, owner_room_id, locator, size, ref_count
                    FROM content_blobs
                    WHERE hash = $1
                    ORDER BY ref_count DESC, id ASC
                    LIMIT 1
                    "#,
                )
                .bind(hash)
                .fetch_optional(self.pool.as_ref())
                .await?
            }
        };
        match row {
            Some(row) => Ok(Some(parse_blob(&row)?)),
            None => Ok(None),
        }
    }

    async fn upsert_ref(&self, blob: ContentBlob) -> anyhow::Result<ContentBlob> {
        let row = sqlx::query(
            r#"
            INSERT INTO content_blobs
                (hash, owner_room_id, locator, size, ref_count, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 1, $5, $6)
            ON CONFLICT(hash, owner_room_id) DO UPDATE
                SET ref_count = content_blobs.ref_count + 1, updated_at = $6
            RETURNING hash, owner_room_id, locator, size, ref_count
            "#,
        )
        .bind(&blob.hash)
        .bind(blob.owner_room_id)
        .bind(&blob.locator)
        .bind(blob.size)
        .bind(now_str())
        .bind(now_str())
        .fetch_one(self.pool.as_ref())
        .await?;
        Ok(parse_blob(&row)?)
    }

    async fn decrement_for(&self, room_id: i64, hash: &str) -> anyhow::Result<Option<ContentBlob>> {
        let target = self.find_by_hash(hash, Some(room_id)).await?;
        let target = match target {
            Some(blob) => Some(blob),
            None => self.find_by_hash(hash, None).await?,
        };
        let Some(blob) = target else {
            return Ok(None);
        };

        sqlx::query("UPDATE content_blobs SET ref_count = ref_count - 1, updated_at = $2 WHERE hash = $1 AND owner_room_id = $3")
            .bind(hash)
            .bind(now_str())
            .bind(blob.owner_room_id)
            .execute(self.pool.as_ref())
            .await?;

        if blob.ref_count > 1 {
            return Ok(None);
        }
        sqlx::query("DELETE FROM content_blobs WHERE hash = $1 AND owner_room_id = $2")
            .bind(hash)
            .bind(blob.owner_room_id)
            .execute(self.pool.as_ref())
            .await?;
        Ok(Some(ContentBlob {
            ref_count: 0,
            ..blob
        }))
    }

    async fn release_room(
        &self,
        entries: &[(String, i64)],
        room_id: i64,
    ) -> anyhow::Result<Vec<ContentBlob>> {
        let mut zeroed = Vec::new();
        for (hash, count) in entries {
            // 本房间对同一哈希可能有多个内容行（同文件多次上传），逐一递减。
            for _ in 0..*count {
                match self.decrement_for(room_id, hash).await? {
                    Some(blob) => zeroed.push(blob),
                    None => break,
                }
            }
        }
        Ok(zeroed)
    }

    async fn count_by_owner(&self, room_id: i64) -> anyhow::Result<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM content_blobs WHERE owner_room_id = $1")
                .bind(room_id)
                .fetch_one(self.pool.as_ref())
                .await?;
        Ok(count)
    }
}
