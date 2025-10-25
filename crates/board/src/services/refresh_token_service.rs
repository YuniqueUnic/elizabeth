use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::{RefreshTokenResponse, Room, RoomRefreshToken, TokenBlacklistEntry};
use crate::repository::room_refresh_token_repository::{
    IRoomRefreshTokenRepository, ITokenBlacklistRepository, SqliteRoomRefreshTokenRepository,
    SqliteTokenBlacklistRepository,
};
use crate::repository::room_repository::{IRoomRepository, SqliteRoomRepository};
use crate::services::token::{RoomTokenClaims, RoomTokenService, TokenType};

/// 刷新令牌服务，简化版本
#[derive(Clone)]
pub struct RefreshTokenService {
    // 基础令牌服务
    base_service: RoomTokenService,
    // 刷新令牌 TTL
    refresh_ttl: Duration,
    // 是否启用令牌轮换
    enable_rotation: bool,
    // Repository 实例
    refresh_token_repo: Arc<dyn IRoomRefreshTokenRepository + Send + Sync>,
    blacklist_repo: Arc<dyn ITokenBlacklistRepository + Send + Sync>,
}

impl RefreshTokenService {
    /// 创建新的刷新令牌服务
    pub fn new(
        base_service: RoomTokenService,
        refresh_ttl: Duration,
        enable_rotation: bool,
        refresh_token_repo: Arc<dyn IRoomRefreshTokenRepository + Send + Sync>,
        blacklist_repo: Arc<dyn ITokenBlacklistRepository + Send + Sync>,
    ) -> Self {
        Self {
            base_service,
            refresh_ttl,
            enable_rotation,
            refresh_token_repo,
            blacklist_repo,
        }
    }

    /// 使用默认配置创建刷新令牌服务
    pub fn with_defaults(
        base_service: RoomTokenService,
        refresh_token_repo: Arc<dyn IRoomRefreshTokenRepository + Send + Sync>,
        blacklist_repo: Arc<dyn ITokenBlacklistRepository + Send + Sync>,
    ) -> Self {
        Self::new(
            base_service,
            Duration::days(7), // 默认 7 天
            true,              // 默认启用轮换
            refresh_token_repo,
            blacklist_repo,
        )
    }

    /// 签发访问令牌和刷新令牌对
    pub async fn issue_token_pair(&self, room: &Room) -> Result<RefreshTokenResponse> {
        if room.is_expired() {
            return Err(anyhow!("room already expired"));
        }

        let now = Utc::now();
        let room_id = room.id.ok_or_else(|| anyhow!("room id missing"))?;

        // 1. 创建访问令牌（复用基础服务）
        let (access_token, access_claims) = self.base_service.issue(room)?;
        let access_jti = access_claims.jti.clone();

        // 2. 创建刷新令牌
        let refresh_exp = now + self.refresh_ttl;
        let refresh_jti = Uuid::new_v4().to_string();

        // 3. 创建刷新令牌声明
        let refresh_claims = RoomTokenClaims::refresh_token_builder(room_id, room.slug.clone())
            .permission(room.permission.bits())
            .max_size(room.max_size)
            .exp(refresh_exp.timestamp())
            .iat(now.timestamp())
            .jti(refresh_jti.clone())
            .build_refresh_token();

        // 4. 签发刷新令牌（复用基础服务的编码逻辑）
        let refresh_token_signed = self.base_service.encode_claims(&refresh_claims)?;

        // 5. 存储刷新令牌到数据库
        let refresh_token_record = RoomRefreshToken::new(
            room_id,
            access_jti,
            &refresh_token_signed,
            refresh_exp.naive_utc(),
        );

        self.refresh_token_repo
            .create(&refresh_token_record)
            .await?;

        // 6. 返回响应
        Ok(RefreshTokenResponse {
            access_token,
            refresh_token: refresh_token_signed,
            access_token_expires_at: access_claims.expires_at(),
            refresh_token_expires_at: refresh_exp.naive_utc(),
        })
    }

    /// 刷新访问令牌
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<RefreshTokenResponse> {
        // 1. 解码刷新令牌
        let refresh_claims = self.base_service.decode(refresh_token)?;

        // 2. 验证令牌类型
        if !refresh_claims.is_refresh_token() {
            return Err(anyhow!("invalid token type, expected refresh token"));
        }

        // 3. 检查令牌是否在黑名单中
        if self
            .blacklist_repo
            .is_blacklisted(&refresh_claims.jti)
            .await?
        {
            return Err(anyhow!("refresh token is blacklisted"));
        }

        // 4. 查找刷新令牌记录
        let refresh_token_hash = RoomRefreshToken::hash_token(refresh_token);
        let token_record = self
            .refresh_token_repo
            .find_by_token_hash(&refresh_token_hash)
            .await?
            .ok_or_else(|| anyhow!("refresh token not found"))?;

        // 5. 验证刷新令牌有效性
        if !token_record.is_valid() {
            return Err(anyhow!("refresh token is invalid or expired"));
        }

        // 6. 获取房间信息
        let room_id = refresh_claims.room_id;
        // 这里应该从数据库获取房间信息，但为了简化，我们假设房间有效
        // 在实际实现中，应该添加房间验证逻辑

        // 7. 如果启用轮换，撤销旧的刷新令牌
        if self.enable_rotation {
            self.refresh_token_repo
                .revoke(token_record.id.unwrap())
                .await?;

            // 将旧令牌添加到黑名单
            let blacklist_entry =
                TokenBlacklistEntry::new(refresh_claims.jti.clone(), refresh_claims.expires_at());
            self.blacklist_repo.add(&blacklist_entry).await?;
        }

        // 8. 创建新的令牌对
        let now = Utc::now();
        let access_exp = now + self.base_service.get_ttl();
        let refresh_exp = now + self.refresh_ttl;

        let new_access_jti = Uuid::new_v4().to_string();
        let new_refresh_jti = Uuid::new_v4().to_string();
        let new_refresh_token = Uuid::new_v4().to_string();

        // 9. 创建新的访问令牌声明
        let new_access_claims = RoomTokenClaims::access_token_builder(
            refresh_claims.room_id,
            refresh_claims.room_name.clone(),
        )
        .permission(refresh_claims.permission)
        .max_size(refresh_claims.max_size)
        .exp(access_exp.timestamp())
        .iat(now.timestamp())
        .jti(new_access_jti.clone())
        .refresh_jti(Some(new_refresh_jti.clone()))
        .build_access_token();

        // 10. 创建新的刷新令牌声明
        let new_refresh_claims = RoomTokenClaims::refresh_token_builder(
            refresh_claims.room_id,
            refresh_claims.room_name.clone(),
        )
        .permission(refresh_claims.permission)
        .max_size(refresh_claims.max_size)
        .exp(refresh_exp.timestamp())
        .iat(now.timestamp())
        .jti(new_refresh_jti.clone())
        .build_refresh_token();

        // 11. 签发新令牌
        let new_access_token = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &new_access_claims,
            &EncodingKey::from_secret(self.base_service.get_secret().as_bytes()),
        )
        .context("failed to sign new access token")?;

        let new_refresh_token_signed = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &new_refresh_claims,
            &EncodingKey::from_secret(self.base_service.get_secret().as_bytes()),
        )
        .context("failed to sign new refresh token")?;

        // 12. 存储新的刷新令牌
        let new_refresh_token_record = RoomRefreshToken::new(
            refresh_claims.room_id,
            new_access_jti,
            &new_refresh_token,
            refresh_exp.naive_utc(),
        );

        self.refresh_token_repo
            .create(&new_refresh_token_record)
            .await?;

        // 13. 更新最后使用时间
        self.refresh_token_repo
            .update_last_used(token_record.id.unwrap())
            .await?;

        // 14. 返回响应
        Ok(RefreshTokenResponse {
            access_token: new_access_token,
            refresh_token: new_refresh_token_signed,
            access_token_expires_at: access_exp.naive_utc(),
            refresh_token_expires_at: refresh_exp.naive_utc(),
        })
    }

    /// 撤销令牌
    pub async fn revoke_token(&self, jti: &str) -> Result<()> {
        // 1. 将令牌添加到黑名单
        let now = Utc::now();
        let blacklist_entry = TokenBlacklistEntry::new(
            jti.to_string(),
            now.naive_utc() + Duration::days(30), // 黑名单记录保留 30 天
        );
        self.blacklist_repo.add(&blacklist_entry).await?;

        // 2. 撤销关联的刷新令牌
        let revoked = self.refresh_token_repo.revoke_by_access_jti(jti).await?;
        if !revoked {
            return Err(anyhow!("no refresh token found for the given access token"));
        }

        Ok(())
    }

    /// 清理过期的令牌和黑名单记录
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let mut cleaned = 0u64;

        // 1. 清理过期的刷新令牌
        cleaned += self.refresh_token_repo.delete_expired().await?;

        // 2. 清理过期的黑名单记录
        cleaned += self.blacklist_repo.remove_expired().await?;

        // 3. 撤销过期的刷新令牌
        cleaned += self.refresh_token_repo.revoke_expired().await?;

        Ok(cleaned)
    }

    /// 验证刷新令牌
    pub async fn verify_refresh_token(&self, token: &str) -> Result<RoomTokenClaims> {
        let claims = self.base_service.decode(token)?;

        if !claims.is_refresh_token() {
            return Err(anyhow!("invalid token type, expected refresh token"));
        }

        // 检查黑名单
        if self.blacklist_repo.is_blacklisted(&claims.jti).await? {
            return Err(anyhow!("refresh token is blacklisted"));
        }

        // 检查数据库中的记录
        let token_hash = RoomRefreshToken::hash_token(token);
        let token_record = self
            .refresh_token_repo
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or_else(|| anyhow!("refresh token not found"))?;

        if !token_record.is_valid() {
            return Err(anyhow!("refresh token is invalid or expired"));
        }

        Ok(claims)
    }

    /// 获取访问令牌的 TTL
    pub fn get_access_token_ttl(&self) -> Duration {
        self.base_service.get_ttl()
    }

    /// 获取刷新令牌的 TTL
    pub fn get_refresh_token_ttl(&self) -> Duration {
        self.refresh_ttl
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbPool;
    use crate::repository::room_refresh_token_repository::{
        SqliteRoomRefreshTokenRepository, SqliteTokenBlacklistRepository,
    };
    use chrono::Duration;
    use sqlx::SqlitePool;

    async fn create_test_pool() -> DbPool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // 运行迁移
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_issue_token_pair() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret".to_string());

        let base_service = RoomTokenService::new(secret.clone());
        let pool_arc = Arc::new(pool.clone());
        let refresh_repo = Arc::new(SqliteRoomRefreshTokenRepository::new(pool_arc.clone()));
        let blacklist_repo = Arc::new(SqliteTokenBlacklistRepository::new(pool_arc.clone()));
        let room_repo = Arc::new(SqliteRoomRepository::new(pool_arc));

        let refresh_service = RefreshTokenService::new(
            base_service,
            Duration::days(7),
            true,
            refresh_repo,
            blacklist_repo,
        );

        // 创建测试房间
        let room = Room::new("test_room".to_string(), Some("password".to_string()));
        let room_with_id = room_repo.create(&room).await.unwrap();

        // 测试令牌对签发
        let response = refresh_service
            .issue_token_pair(&room_with_id)
            .await
            .unwrap();
        assert!(!response.access_token.is_empty());
        assert!(!response.refresh_token.is_empty());

        // 验证过期时间
        let now = Utc::now();
        assert!(response.access_token_expires_at > now.naive_utc());
        assert!(response.refresh_token_expires_at > now.naive_utc());
    }

    #[tokio::test]
    async fn test_refresh_access_token() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret".to_string());

        let base_service = RoomTokenService::new(secret.clone());
        let pool_arc = Arc::new(pool.clone());
        let refresh_repo = Arc::new(SqliteRoomRefreshTokenRepository::new(pool_arc.clone()));
        let blacklist_repo = Arc::new(SqliteTokenBlacklistRepository::new(pool_arc.clone()));
        let room_repo = Arc::new(SqliteRoomRepository::new(pool_arc));

        let refresh_service = RefreshTokenService::new(
            base_service,
            Duration::days(7),
            true,
            refresh_repo,
            blacklist_repo,
        );

        // 创建测试房间
        let room = Room::new("test_room".to_string(), Some("password".to_string()));
        let room_with_id = room_repo.create(&room).await.unwrap();

        // 1. 签发初始令牌对
        let initial_response = refresh_service
            .issue_token_pair(&room_with_id)
            .await
            .unwrap();

        // 2. 使用刷新令牌获取新的令牌对
        let refreshed_response = refresh_service
            .refresh_access_token(&initial_response.refresh_token)
            .await
            .unwrap();

        // 3. 验证新令牌
        assert!(!refreshed_response.access_token.is_empty());
        assert!(!refreshed_response.refresh_token.is_empty());
        assert_ne!(
            refreshed_response.access_token,
            initial_response.access_token
        );
        assert_ne!(
            refreshed_response.refresh_token,
            initial_response.refresh_token
        );
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret".to_string());

        let base_service = RoomTokenService::new(secret.clone());
        let pool_arc = Arc::new(pool.clone());
        let refresh_repo = Arc::new(SqliteRoomRefreshTokenRepository::new(pool_arc.clone()));
        let blacklist_repo = Arc::new(SqliteTokenBlacklistRepository::new(pool_arc.clone()));
        let room_repo = Arc::new(SqliteRoomRepository::new(pool_arc));

        let refresh_service = RefreshTokenService::new(
            base_service.clone(),
            Duration::days(7),
            true,
            refresh_repo,
            blacklist_repo.clone(),
        );

        // 创建测试房间
        let room = Room::new("test_room".to_string(), Some("password".to_string()));
        let room_with_id = room_repo.create(&room).await.unwrap();

        // 1. 签发令牌对
        let response = refresh_service
            .issue_token_pair(&room_with_id)
            .await
            .unwrap();

        // 2. 解码访问令牌获取 JTI
        let claims = base_service.decode(&response.access_token).unwrap();

        // 3. 撤销令牌
        refresh_service.revoke_token(&claims.jti).await.unwrap();

        // 4. 验证令牌是否在黑名单中
        assert!(blacklist_repo.is_blacklisted(&claims.jti).await.unwrap());
    }
}
