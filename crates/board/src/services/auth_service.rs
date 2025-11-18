use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{debug, error, info, warn};

use crate::models::Room;
use crate::repository::room_refresh_token_repository::ITokenBlacklistRepository;
use crate::services::RoomTokenClaims;
use crate::services::token::RoomTokenService;

/// 认证服务，集成令牌验证和黑名单检查
#[derive(Clone)]
pub struct AuthService {
    token_service: Arc<RoomTokenService>,
    blacklist_repo: Arc<dyn ITokenBlacklistRepository + Send + Sync>,
}

impl AuthService {
    /// 创建新的认证服务
    pub fn new(
        token_service: Arc<RoomTokenService>,
        blacklist_repo: Arc<dyn ITokenBlacklistRepository + Send + Sync>,
    ) -> Self {
        Self {
            token_service,
            blacklist_repo,
        }
    }

    /// 验证访问令牌
    /// 1. 解码令牌
    /// 2. 检查黑名单
    /// 3. 验证房间状态
    /// 4. 返回声明
    pub async fn verify_access_token(&self, token: &str, room: &Room) -> Result<RoomTokenClaims> {
        // 1. 解码令牌
        let claims = self
            .token_service
            .decode(token)
            .context("failed to decode access token")?;

        // 2. 验证令牌类型
        if !claims.is_access_token() {
            return Err(anyhow!("invalid token type, expected access token"));
        }

        // 3. 检查令牌是否在黑名单中
        if self.blacklist_repo.is_blacklisted(&claims.jti).await? {
            return Err(anyhow!("access token is blacklisted"));
        }

        // 4. 检查令牌是否过期
        if claims.is_expired() {
            return Err(anyhow!("access token is expired"));
        }

        // 5. 验证房间 ID 匹配
        if claims.room_id != room.id.unwrap_or_default() {
            return Err(anyhow!("token room_id does not match requested room"));
        }

        // 6. 检查房间状态
        if room.is_expired() {
            return Err(anyhow!("room is expired"));
        }

        if room.status() != crate::models::RoomStatus::Open {
            return Err(anyhow!("room is not open"));
        }

        if !room.can_enter() {
            return Err(anyhow!("room cannot be entered"));
        }

        Ok(claims)
    }

    /// 验证刷新令牌
    /// 1. 解码令牌
    /// 2. 检查黑名单
    /// 3. 验证令牌类型
    /// 4. 返回声明
    pub async fn verify_refresh_token(&self, token: &str) -> Result<RoomTokenClaims> {
        // 1. 解码令牌
        let claims = self
            .token_service
            .decode(token)
            .context("failed to decode refresh token")?;

        // 2. 验证令牌类型
        if !claims.is_refresh_token() {
            return Err(anyhow!("invalid token type, expected refresh token"));
        }

        // 3. 检查令牌是否在黑名单中
        if self.blacklist_repo.is_blacklisted(&claims.jti).await? {
            return Err(anyhow!("refresh token is blacklisted"));
        }

        // 4. 检查令牌是否过期
        if claims.is_expired() {
            return Err(anyhow!("refresh token is expired"));
        }

        Ok(claims)
    }

    /// 将令牌添加到黑名单
    pub async fn blacklist_token(&self, claims: &RoomTokenClaims) -> Result<()> {
        let expires_at = claims.expires_at();
        let blacklist_entry =
            crate::models::TokenBlacklistEntry::new(claims.jti.clone(), expires_at);

        self.blacklist_repo.add(&blacklist_entry).await?;
        Ok(())
    }

    /// 检查令牌是否在黑名单中
    pub async fn is_token_blacklisted(&self, jti: &str) -> Result<bool> {
        self.blacklist_repo.is_blacklisted(jti).await
    }

    /// 获取令牌服务
    pub fn get_token_service(&self) -> &Arc<RoomTokenService> {
        &self.token_service
    }

    /// 获取黑名单仓库
    pub fn get_blacklist_repo(&self) -> &Arc<dyn ITokenBlacklistRepository + Send + Sync> {
        &self.blacklist_repo
    }

    /// 清理过期的黑名单记录
    pub async fn cleanup_blacklist(&self) -> Result<u64> {
        self.blacklist_repo.remove_expired().await
    }

    /// 验证令牌并检查房间权限
    /// 这是核心权限验证方法，其他所有权限验证都应该基于这个方法
    pub async fn verify_token_with_room_permission(
        &self,
        token: &str,
        room: &Room,
        required_permission: crate::models::room::permission::RoomPermission,
    ) -> Result<RoomTokenClaims> {
        let claims = self.verify_access_token(token, room).await?;

        let user_permission = claims.as_permission();
        if !user_permission.contains(required_permission) {
            return Err(anyhow!("insufficient permissions"));
        }

        Ok(claims)
    }

    /// 从授权头中提取令牌
    pub fn extract_token_from_header(&self, auth_header: &str) -> Result<String> {
        if !auth_header.to_lowercase().starts_with("bearer ") {
            return Err(anyhow!("invalid authorization header format"));
        }

        let token = auth_header[7..].trim();
        if token.is_empty() {
            return Err(anyhow!("empty token in authorization header"));
        }

        Ok(token.to_string())
    }

    /// 验证授权头中的令牌
    pub async fn verify_auth_header(
        &self,
        auth_header: &str,
        room: &Room,
    ) -> Result<RoomTokenClaims> {
        let token = self.extract_token_from_header(auth_header)?;
        self.verify_access_token(&token, room).await
    }

    /// 验证授权头中的令牌并检查权限
    /// 核心授权头权限验证方法
    pub async fn verify_auth_header_with_permission(
        &self,
        auth_header: &str,
        room: &Room,
        required_permission: crate::models::room::permission::RoomPermission,
    ) -> Result<RoomTokenClaims> {
        let token = self.extract_token_from_header(auth_header)?;
        self.verify_token_with_room_permission(&token, room, required_permission)
            .await
    }

    /// 检查令牌是否即将过期（5 分钟内）
    pub async fn is_token_expiring_soon(&self, token: &str) -> Result<bool> {
        let claims = self.token_service.decode(token)?;
        Ok(claims.is_expiring_soon())
    }

    /// 获取令牌剩余有效时间（秒）
    pub async fn get_token_remaining_seconds(&self, token: &str) -> Result<i64> {
        let claims = self.token_service.decode(token)?;
        Ok(claims.remaining_seconds())
    }

    /// 获取令牌年龄（秒）
    pub async fn get_token_age_seconds(&self, token: &str) -> Result<i64> {
        let claims = self.token_service.decode(token)?;
        Ok(claims.age_seconds())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DbPoolSettings, run_migrations};
    use crate::repository::TokenBlacklistRepository;
    use crate::services::RoomTokenClaims;
    use crate::services::token::RoomTokenService;
    use chrono::Duration;
    use std::sync::Arc;

    async fn create_test_pool() -> Arc<crate::db::DbPool> {
        let url = "sqlite::memory:";
        let pool = DbPoolSettings::new(url).create_pool().await.unwrap();
        run_migrations(&pool, url).await.unwrap();
        Arc::new(pool)
    }

    #[tokio::test]
    async fn test_extract_token_from_header() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret".to_string());

        let token_service = Arc::new(RoomTokenService::new(secret.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(pool.clone()));

        let auth_service = AuthService::new(token_service, blacklist_repo);

        // 测试有效的 Bearer 令牌
        let valid_header = "Bearer abc123";
        let token = auth_service
            .extract_token_from_header(valid_header)
            .unwrap();
        assert_eq!(token, "abc123");

        // 测试无效的令牌格式
        let invalid_header = "Basic abc123";
        let result = auth_service.extract_token_from_header(invalid_header);
        assert!(result.is_err());

        // 测试空令牌
        let empty_header = "Bearer ";
        let result = auth_service.extract_token_from_header(empty_header);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_blacklist_token() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret".to_string());

        let token_service = Arc::new(RoomTokenService::new(secret.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(pool.clone()));

        let auth_service = AuthService::new(token_service, blacklist_repo);

        // 创建测试令牌声明
        let now = Utc::now();
        let claims = RoomTokenClaims {
            sub: "room:1".to_string(),
            room_id: 0,
            room_name: "test_room".to_string(),
            permission: 0,
            max_size: 1024,
            exp: (now + Duration::hours(1)).timestamp(),
            iat: now.timestamp(),
            jti: "test_jti".to_string(),
            token_type: crate::services::token::TokenType::Access,
            refresh_jti: None,
        };

        // 测试将令牌添加到黑名单
        auth_service.blacklist_token(&claims).await.unwrap();

        // 验证令牌是否在黑名单中
        assert!(auth_service.is_token_blacklisted("test_jti").await.unwrap());

        // 验证不在黑名单中的令牌
        assert!(
            !auth_service
                .is_token_blacklisted("unknown_jti")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_verify_access_token_success() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret_for_unit_testing".to_string());

        let token_service = Arc::new(RoomTokenService::new(secret.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(pool.clone()));

        let auth_service = AuthService::new(token_service.clone(), blacklist_repo);

        // 创建有效的访问令牌声明
        let room = crate::models::Room::new("test_room".to_string(), None);

        let claims = RoomTokenClaims {
            sub: "room:test_room".to_string(),
            room_id: 0,
            room_name: "test_room".to_string(),
            permission: crate::models::room::permission::RoomPermission::VIEW_ONLY.bits(),
            max_size: 1024,
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: crate::services::token::TokenType::Access,
            refresh_jti: None,
        };

        // 生成令牌
        let token = token_service.encode_claims(&claims).unwrap();

        // 验证访问令牌
        let verified_claims = auth_service
            .verify_access_token(&token, &room)
            .await
            .unwrap();
        assert_eq!(verified_claims.room_name, "test_room");
        assert_eq!(verified_claims.room_id, 0);
        assert!(verified_claims.is_access_token());
    }

    #[tokio::test]
    async fn test_verify_access_token_blacklisted() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret_for_unit_testing".to_string());

        let token_service = Arc::new(RoomTokenService::new(secret.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(pool.clone()));

        let auth_service = AuthService::new(token_service.clone(), blacklist_repo.clone());

        // 创建访问令牌声明
        let room = crate::models::Room::new("test_room".to_string(), None);

        let claims = RoomTokenClaims {
            sub: "room:test_room".to_string(),
            room_id: 0,
            room_name: "test_room".to_string(),
            permission: crate::models::room::permission::RoomPermission::VIEW_ONLY.bits(),
            max_size: 1024,
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: crate::services::token::TokenType::Access,
            refresh_jti: None,
        };

        // 生成令牌
        let token = token_service.encode_claims(&claims).unwrap();

        // 将令牌加入黑名单
        auth_service.blacklist_token(&claims).await.unwrap();

        // 验证黑名单令牌应该失败
        let result = auth_service.verify_access_token(&token, &room).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("blacklisted"));
    }

    #[tokio::test]
    async fn test_verify_refresh_token_success() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret_for_unit_testing".to_string());

        let token_service = Arc::new(RoomTokenService::new(secret.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(pool.clone()));

        let auth_service = AuthService::new(token_service.clone(), blacklist_repo);

        // 创建刷新令牌声明
        let claims = RoomTokenClaims {
            sub: "room:test_room".to_string(),
            room_id: 0,
            room_name: "test_room".to_string(),
            permission: crate::models::room::permission::RoomPermission::VIEW_ONLY.bits(),
            max_size: 1024,
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: crate::services::token::TokenType::Refresh,
            refresh_jti: None,
        };

        // 生成刷新令牌
        let refresh_token = token_service.encode_claims(&claims).unwrap();

        // 验证刷新令牌
        let verified_claims = auth_service
            .verify_refresh_token(&refresh_token)
            .await
            .unwrap();
        assert_eq!(verified_claims.room_name, "test_room");
        assert!(verified_claims.is_refresh_token());
    }

    #[tokio::test]
    async fn test_token_expiration_checks() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret_for_unit_testing".to_string());

        let token_service = Arc::new(RoomTokenService::new(secret.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(pool.clone()));

        let auth_service = AuthService::new(token_service.clone(), blacklist_repo);

        // 创建短期令牌声明（1 分钟过期）
        let short_lived_claims = RoomTokenClaims {
            sub: "room:test_room".to_string(),
            room_id: 0,
            room_name: "test_room".to_string(),
            permission: crate::models::room::permission::RoomPermission::VIEW_ONLY.bits(),
            max_size: 1024,
            exp: (Utc::now() + chrono::Duration::minutes(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: crate::services::token::TokenType::Access,
            refresh_jti: None,
        };

        // 生成令牌
        let short_lived_token = token_service.encode_claims(&short_lived_claims).unwrap();

        // 测试令牌剩余时间
        let remaining_seconds = auth_service
            .get_token_remaining_seconds(&short_lived_token)
            .await
            .unwrap();
        assert!(remaining_seconds > 0);
        assert!(remaining_seconds <= 60);

        // 测试令牌年龄
        let age_seconds = auth_service
            .get_token_age_seconds(&short_lived_token)
            .await
            .unwrap();
        assert!(age_seconds >= 0);
        assert!(age_seconds < 5); // 应该很小，因为刚创建

        // 创建即将过期的令牌声明（4 分钟过期，应该触发"即将过期"检查）
        let expiring_claims = RoomTokenClaims {
            sub: "room:test_room".to_string(),
            room_id: 0,
            room_name: "test_room".to_string(),
            permission: crate::models::room::permission::RoomPermission::VIEW_ONLY.bits(),
            max_size: 1024,
            exp: (Utc::now() + chrono::Duration::minutes(4)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: crate::services::token::TokenType::Access,
            refresh_jti: None,
        };

        // 生成令牌
        let expiring_token = token_service.encode_claims(&expiring_claims).unwrap();

        // 测试即将过期检查
        let is_expiring_soon = auth_service
            .is_token_expiring_soon(&expiring_token)
            .await
            .unwrap();
        assert!(is_expiring_soon); // 4 分钟应该触发"即将过期"（5 分钟阈值）
    }

    #[tokio::test]
    async fn test_getters() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret".to_string());

        let token_service = Arc::new(RoomTokenService::new(secret.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(Arc::new(pool)));

        let auth_service = AuthService::new(token_service.clone(), blacklist_repo.clone());

        // 测试 getter 方法
        assert_eq!(
            auth_service.get_token_service().as_ref() as *const _,
            token_service.as_ref() as *const _
        );
        assert_eq!(
            auth_service.get_blacklist_repo().as_ref() as *const _,
            blacklist_repo.as_ref() as *const _
        );
    }

    #[tokio::test]
    async fn test_cleanup_blacklist() {
        let pool = create_test_pool().await;
        let secret = Arc::new("test_secret".to_string());

        let token_service = Arc::new(RoomTokenService::new(secret.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(Arc::new(pool)));

        let auth_service = AuthService::new(token_service, blacklist_repo);

        // 测试清理黑名单（应该能正常调用）
        auth_service.cleanup_blacklist().await.unwrap();
    }
}
