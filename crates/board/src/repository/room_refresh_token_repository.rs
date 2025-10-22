use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, Utc};
use log::{debug, error, info, warn};
use sqlx::{Executor, Sqlite};
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::{RoomRefreshToken, TokenBlacklistEntry};

#[async_trait]
pub trait IRoomRefreshTokenRepository: Send + Sync {
    async fn create(&self, refresh_token: &RoomRefreshToken) -> Result<RoomRefreshToken>;
    async fn find_by_id(&self, id: i64) -> Result<Option<RoomRefreshToken>>;
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RoomRefreshToken>>;
    async fn find_by_access_jti(&self, access_jti: &str) -> Result<Option<RoomRefreshToken>>;
    async fn find_by_room(&self, room_id: i64) -> Result<Vec<RoomRefreshToken>>;
    async fn find_active_by_room(&self, room_id: i64) -> Result<Vec<RoomRefreshToken>>;
    async fn update_last_used(&self, id: i64) -> Result<bool>;
    async fn revoke(&self, id: i64) -> Result<bool>;
    async fn revoke_by_access_jti(&self, access_jti: &str) -> Result<bool>;
    async fn revoke_expired(&self) -> Result<u64>;
    async fn delete_by_room(&self, room_id: i64) -> Result<u64>;
    async fn delete_expired(&self) -> Result<u64>;
}

pub struct SqliteRoomRefreshTokenRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomRefreshTokenRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional_by_id<'e, E>(executor: E, id: i64) -> Result<Option<RoomRefreshToken>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let token = sqlx::query_as!(
            RoomRefreshToken,
            r#"
            SELECT
                id,
                room_id,
                access_token_jti,
                token_hash,
                expires_at,
                created_at,
                last_used_at,
                is_revoked
            FROM room_refresh_tokens
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(token)
    }

    async fn fetch_optional_by_token_hash<'e, E>(
        executor: E,
        token_hash: &str,
    ) -> Result<Option<RoomRefreshToken>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let token = sqlx::query_as!(
            RoomRefreshToken,
            r#"
            SELECT
                id,
                room_id,
                access_token_jti,
                token_hash,
                expires_at,
                created_at,
                last_used_at,
                is_revoked
            FROM room_refresh_tokens
            WHERE token_hash = ?
            "#,
            token_hash
        )
        .fetch_optional(executor)
        .await?;

        Ok(token)
    }

    async fn fetch_optional_by_access_jti<'e, E>(
        executor: E,
        access_jti: &str,
    ) -> Result<Option<RoomRefreshToken>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let token = sqlx::query_as!(
            RoomRefreshToken,
            r#"
            SELECT
                id,
                room_id,
                access_token_jti,
                token_hash,
                expires_at,
                created_at,
                last_used_at,
                is_revoked
            FROM room_refresh_tokens
            WHERE access_token_jti = ?
            "#,
            access_jti
        )
        .fetch_optional(executor)
        .await?;

        Ok(token)
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<RoomRefreshToken>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as!(
            RoomRefreshToken,
            r#"
            SELECT
                id,
                room_id,
                access_token_jti,
                token_hash,
                expires_at,
                created_at,
                last_used_at,
                is_revoked
            FROM room_refresh_tokens
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| anyhow!("room refresh token not found for id {}", id))
    }
}

#[async_trait]
impl IRoomRefreshTokenRepository for SqliteRoomRefreshTokenRepository {
    async fn create(&self, refresh_token: &RoomRefreshToken) -> Result<RoomRefreshToken> {
        let mut tx = self.pool.begin().await?;

        let insert = sqlx::query!(
            r#"
            INSERT INTO room_refresh_tokens (
                room_id, access_token_jti, token_hash, expires_at,
                created_at, last_used_at, is_revoked
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            refresh_token.room_id,
            refresh_token.access_token_jti,
            refresh_token.token_hash,
            refresh_token.expires_at,
            refresh_token.created_at,
            refresh_token.last_used_at,
            refresh_token.is_revoked
        )
        .execute(&mut *tx)
        .await?;

        let token = Self::fetch_by_id_or_err(&mut *tx, insert.last_insert_rowid()).await?;
        tx.commit().await?;
        Ok(token)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<RoomRefreshToken>> {
        Self::fetch_optional_by_id(&*self.pool, id).await
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RoomRefreshToken>> {
        Self::fetch_optional_by_token_hash(&*self.pool, token_hash).await
    }

    async fn find_by_access_jti(&self, access_jti: &str) -> Result<Option<RoomRefreshToken>> {
        Self::fetch_optional_by_access_jti(&*self.pool, access_jti).await
    }

    async fn find_by_room(&self, room_id: i64) -> Result<Vec<RoomRefreshToken>> {
        let tokens = sqlx::query_as!(
            RoomRefreshToken,
            r#"
            SELECT
                id,
                room_id,
                access_token_jti,
                token_hash,
                expires_at,
                created_at,
                last_used_at,
                is_revoked
            FROM room_refresh_tokens
            WHERE room_id = ?
            ORDER BY created_at DESC
            "#,
            room_id
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(tokens)
    }

    async fn find_active_by_room(&self, room_id: i64) -> Result<Vec<RoomRefreshToken>> {
        let now = Utc::now().naive_utc();
        let tokens = sqlx::query_as!(
            RoomRefreshToken,
            r#"
            SELECT
                id,
                room_id,
                access_token_jti,
                token_hash,
                expires_at,
                created_at,
                last_used_at,
                is_revoked
            FROM room_refresh_tokens
            WHERE room_id = ?
              AND expires_at > ?
              AND is_revoked = FALSE
            ORDER BY created_at DESC
            "#,
            room_id,
            now
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(tokens)
    }

    async fn update_last_used(&self, id: i64) -> Result<bool> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query!(
            r#"
            UPDATE room_refresh_tokens
            SET last_used_at = ?
            WHERE id = ?
            "#,
            now,
            id
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn revoke(&self, id: i64) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE room_refresh_tokens
            SET is_revoked = TRUE
            WHERE id = ? AND is_revoked = FALSE
            "#,
            id
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn revoke_by_access_jti(&self, access_jti: &str) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE room_refresh_tokens
            SET is_revoked = TRUE
            WHERE access_token_jti = ? AND is_revoked = FALSE
            "#,
            access_jti
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn revoke_expired(&self) -> Result<u64> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query!(
            r#"
            UPDATE room_refresh_tokens
            SET is_revoked = TRUE
            WHERE expires_at <= ? AND is_revoked = FALSE
            "#,
            now
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn delete_by_room(&self, room_id: i64) -> Result<u64> {
        let result = sqlx::query!("DELETE FROM room_refresh_tokens WHERE room_id = ?", room_id)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn delete_expired(&self) -> Result<u64> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query!("DELETE FROM room_refresh_tokens WHERE expires_at <= ?", now)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

#[async_trait]
pub trait ITokenBlacklistRepository: Send + Sync {
    async fn add(&self, entry: &TokenBlacklistEntry) -> Result<TokenBlacklistEntry>;
    async fn find_by_jti(&self, jti: &str) -> Result<Option<TokenBlacklistEntry>>;
    async fn is_blacklisted(&self, jti: &str) -> Result<bool>;
    async fn remove_expired(&self) -> Result<u64>;
    async fn cleanup(&self) -> Result<u64>;
}

pub struct SqliteTokenBlacklistRepository {
    pool: Arc<DbPool>,
}

impl SqliteTokenBlacklistRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional_by_jti<'e, E>(
        executor: E,
        jti: &str,
    ) -> Result<Option<TokenBlacklistEntry>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let entry = sqlx::query_as!(
            TokenBlacklistEntry,
            r#"
            SELECT
                id,
                jti,
                expires_at,
                created_at
            FROM token_blacklist
            WHERE jti = ?
            "#,
            jti
        )
        .fetch_optional(executor)
        .await?;

        Ok(entry)
    }

    async fn fetch_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<TokenBlacklistEntry>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as!(
            TokenBlacklistEntry,
            r#"
            SELECT
                id,
                jti,
                expires_at,
                created_at
            FROM token_blacklist
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| anyhow!("token blacklist entry not found for id {}", id))
    }
}

#[async_trait]
impl ITokenBlacklistRepository for SqliteTokenBlacklistRepository {
    async fn add(&self, entry: &TokenBlacklistEntry) -> Result<TokenBlacklistEntry> {
        let mut tx = self.pool.begin().await?;

        let insert = sqlx::query!(
            r#"
            INSERT INTO token_blacklist (jti, expires_at, created_at)
            VALUES (?, ?, ?)
            "#,
            entry.jti,
            entry.expires_at,
            entry.created_at
        )
        .execute(&mut *tx)
        .await?;

        let entry = Self::fetch_by_id_or_err(&mut *tx, insert.last_insert_rowid()).await?;
        tx.commit().await?;
        Ok(entry)
    }

    async fn find_by_jti(&self, jti: &str) -> Result<Option<TokenBlacklistEntry>> {
        Self::fetch_optional_by_jti(&*self.pool, jti).await
    }

    async fn is_blacklisted(&self, jti: &str) -> Result<bool> {
        let now = Utc::now().naive_utc();
        let entry = sqlx::query_as!(
            TokenBlacklistEntry,
            r#"
            SELECT
                id,
                jti,
                expires_at,
                created_at
            FROM token_blacklist
            WHERE jti = ? AND expires_at > ?
            "#,
            jti,
            now
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(entry.is_some())
    }

    async fn remove_expired(&self) -> Result<u64> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query!("DELETE FROM token_blacklist WHERE expires_at <= ?", now)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn cleanup(&self) -> Result<u64> {
        // 清理过期的黑名单记录
        let removed_blacklist = self.remove_expired().await?;

        // 清理过期的刷新令牌
        let removed_refresh_tokens = self.delete_expired().await?;

        // 撤销过期的刷新令牌
        let revoked_tokens = self.revoke_expired().await?;

        Ok(removed_blacklist + removed_refresh_tokens + revoked_tokens)
    }
}

// 为 SqliteTokenBlacklistRepository 添加 delete_expired 和 revoke_expired 方法
impl SqliteTokenBlacklistRepository {
    async fn delete_expired(&self) -> Result<u64> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query!("DELETE FROM token_blacklist WHERE expires_at <= ?", now)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn revoke_expired(&self) -> Result<u64> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query!(
            r#"
            UPDATE room_refresh_tokens
            SET is_revoked = TRUE
            WHERE expires_at <= ? AND is_revoked = FALSE
            "#,
            now
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use sqlx::SqlitePool;

    async fn create_test_pool() -> DbPool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // 创建测试表
        sqlx::query(
            r#"
            CREATE TABLE room_refresh_tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                room_id INTEGER NOT NULL,
                access_token_jti TEXT NOT NULL,
                token_hash TEXT NOT NULL UNIQUE,
                expires_at DATETIME NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_used_at DATETIME,
                is_revoked BOOLEAN NOT NULL DEFAULT FALSE
            );

            CREATE TABLE token_blacklist (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                jti TEXT NOT NULL UNIQUE,
                expires_at DATETIME NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_refresh_token() {
        let pool = create_test_pool().await;
        let repo = SqliteRoomRefreshTokenRepository::new(Arc::new(pool.clone()));

        let refresh_token = RoomRefreshToken::new(
            1,
            "access_jti".to_string(),
            "refresh_token",
            Utc::now().naive_utc() + Duration::days(7),
        );

        let created = repo.create(&refresh_token).await.unwrap();
        assert!(created.id.is_some());
        assert_eq!(created.room_id, 1);
        assert_eq!(created.access_token_jti, "access_jti");
        assert_ne!(created.token_hash, "refresh_token"); // 应该是哈希值
    }

    #[tokio::test]
    async fn test_find_by_token_hash() {
        let pool = create_test_pool().await;
        let repo = SqliteRoomRefreshTokenRepository::new(Arc::new(pool.clone()));

        let refresh_token = RoomRefreshToken::new(
            1,
            "access_jti".to_string(),
            "refresh_token",
            Utc::now().naive_utc() + Duration::days(7),
        );

        let created = repo.create(&refresh_token).await.unwrap();
        let found = repo.find_by_token_hash(&created.token_hash).await.unwrap();

        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.token_hash, created.token_hash);
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let pool = create_test_pool().await;
        let repo = SqliteRoomRefreshTokenRepository::new(Arc::new(pool.clone()));

        let refresh_token = RoomRefreshToken::new(
            1,
            "access_jti".to_string(),
            "refresh_token",
            Utc::now().naive_utc() + Duration::days(7),
        );

        let created = repo.create(&refresh_token).await.unwrap();
        assert!(!created.is_revoked);

        let revoked = repo.revoke(created.id.unwrap()).await.unwrap();
        assert!(revoked);

        let found = repo.find_by_id(created.id.unwrap()).await.unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().is_revoked);
    }

    #[tokio::test]
    async fn test_token_blacklist() {
        let pool = create_test_pool().await;
        let repo = SqliteTokenBlacklistRepository::new(Arc::new(pool.clone()));

        let entry = TokenBlacklistEntry::new(
            "test_jti".to_string(),
            Utc::now().naive_utc() + Duration::hours(1),
        );

        let created = repo.add(&entry).await.unwrap();
        assert!(created.id.is_some());

        let is_blacklisted = repo.is_blacklisted("test_jti").await.unwrap();
        assert!(is_blacklisted);

        let is_blacklisted = repo.is_blacklisted("unknown_jti").await.unwrap();
        assert!(!is_blacklisted);
    }
}
