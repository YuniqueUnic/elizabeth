use anyhow::Result;
/// 服务模块
///
/// 集中管理所有应用程序服务
use std::sync::Arc;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::repository::room_refresh_token_repository::{
    RoomRefreshTokenRepository, TokenBlacklistRepository,
};
use crate::repository::room_repository::RoomRepository;

pub mod auth_service;
pub mod refresh_token_service;
pub mod room_gc_service;
pub mod token;

// 重新导出服务类型
pub use auth_service::*;
pub use refresh_token_service::*;
pub use room_gc_service::*;
pub use token::*;

/// 服务容器，包含所有应用程序服务
#[derive(Clone)]
pub struct Services {
    pub auth: Arc<AuthService>,
    pub token_service: Arc<RoomTokenService>,
    pub refresh_token_service: Arc<RefreshTokenService>,
    pub room_repository: Arc<RoomRepository>,
    pub room_gc: Arc<RoomGcService>,
}

impl Services {
    /// 创建新的服务容器
    pub fn new(config: &AppConfig, db_pool: Arc<DbPool>) -> Result<Self> {
        // 创建令牌服务
        let token_service = Arc::new(RoomTokenService::with_config(
            Arc::new(config.auth.jwt_secret.clone()),
            config.auth.ttl_seconds,
            config.auth.leeway_seconds,
        ));

        // 创建房间仓库
        let room_repository = Arc::new(RoomRepository::new(db_pool.clone()));

        // 创建刷新令牌仓库
        let refresh_repo = Arc::new(RoomRefreshTokenRepository::new(db_pool.clone()));
        let blacklist_repo = Arc::new(TokenBlacklistRepository::new(db_pool.clone()));

        // 创建刷新令牌服务
        let refresh_token_service = Arc::new(RefreshTokenService::with_defaults(
            (*token_service).clone(),
            refresh_repo,
            blacklist_repo.clone(),
        ));

        // 创建认证服务
        let auth_service = Arc::new(AuthService::new(token_service.clone(), blacklist_repo));

        let room_gc = Arc::new(RoomGcService::new(
            db_pool.clone(),
            config.storage.root.clone(),
        ));

        Ok(Self {
            auth: auth_service,
            token_service,
            refresh_token_service,
            room_repository,
            room_gc,
        })
    }

    /// 获取认证服务的引用
    pub fn auth(&self) -> &AuthService {
        &self.auth
    }

    /// 获取令牌服务的引用
    pub fn token_service(&self) -> &RoomTokenService {
        &self.token_service
    }

    /// 获取刷新令牌服务的引用
    pub fn refresh_token_service(&self) -> &RefreshTokenService {
        &self.refresh_token_service
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthConfig;
    use crate::db::{DbPoolSettings, init_db};

    #[tokio::test]
    async fn test_services_creation() -> Result<()> {
        // 创建测试数据库
        let db_settings = DbPoolSettings::new("sqlite::memory:");
        let db_pool = Arc::new(init_db(&db_settings).await?);

        // 创建测试配置
        let mut config = AppConfig::for_development();
        config.auth = AuthConfig::new("test-secret-key-for-unit-testing-123".to_string())?;

        // 创建服务
        let services = Services::new(&config, db_pool)?;

        // 验证服务创建成功
        assert!(!services.token_service.get_secret().is_empty());

        Ok(())
    }
}
