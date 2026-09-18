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
use crate::repository::{RoomAccessRepository, RoomTokenRepository};

pub mod admin_auth;
pub mod admin_session;
pub mod attempt_guard;
pub mod auth_service;
pub mod password;
pub mod refresh_token_service;
pub mod room_lifecycle;
pub mod token;

// 重新导出服务类型
pub use admin_auth::*;
pub use admin_session::*;
pub use attempt_guard::*;
pub use auth_service::*;
pub use password::*;
pub use refresh_token_service::*;
pub use room_lifecycle::*;
pub use token::*;

/// 服务容器，包含所有应用程序服务
#[derive(Clone)]
pub struct Services {
    pub auth: Arc<AuthService>,
    pub token_service: Arc<RoomTokenService>,
    pub refresh_token_service: Arc<RefreshTokenService>,
    pub room_repository: Arc<RoomRepository>,
    pub room_lifecycle: Arc<RoomLifecycleService>,
    pub password_hash: Arc<PasswordHashService>,
    /// 管理员登录会话（JWT 签发/验签）
    pub admin_session: Arc<AdminSessionService>,
    /// 统一防爆破守卫（身份码 / 房间密码 / 文件兑换码 / 管理登录）
    pub attempt_guard: Arc<AttemptGuard>,
    /// 房间角色矩阵缓存（AppState 与 refresh service 共享同一份）
    pub roles_cache: Arc<crate::authz::RoleTableCache>,
}

impl Services {
    /// 创建新的服务容器
    pub fn new(
        config: &AppConfig,
        db_pool: Arc<DbPool>,
        storage: Arc<dyn crate::storage::StorageBackend>,
        runtime: Arc<crate::state::RuntimeConfigOverrides>,
    ) -> Result<Self> {
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

        // 创建角色矩阵缓存
        let roles_cache = Arc::new(crate::authz::RoleTableCache::new());

        // 创建刷新令牌服务
        let access_repository = RoomAccessRepository::new(db_pool.clone());
        let access_token_repository = Arc::new(RoomTokenRepository::new(db_pool.clone()));
        let refresh_token_service = Arc::new(RefreshTokenService::new(
            (*token_service).clone(),
            chrono::Duration::seconds(config.auth.refresh_ttl_seconds),
            config.auth.enable_refresh_token_rotation,
            db_pool.clone(),
            roles_cache.clone(),
            room_repository.clone(),
            access_repository,
            access_token_repository,
            refresh_repo,
            blacklist_repo.clone(),
            runtime,
        ));

        // 创建认证服务
        let auth_service = Arc::new(AuthService::new(token_service.clone(), blacklist_repo));

        let room_lifecycle_repository = Arc::new(crate::repository::RoomLifecycleRepository::new(
            db_pool.clone(),
        ));
        let room_lifecycle = Arc::new(RoomLifecycleService::new(
            room_lifecycle_repository,
            storage,
            Arc::new(crate::repository::ContentBlobRepository::new(
                db_pool.clone(),
            )),
        ));
        let password_hash = Arc::new(PasswordHashService);
        let admin_session = Arc::new(AdminSessionService::new(
            Arc::new(config.auth.jwt_secret.clone()),
            config.auth.ttl_seconds,
            config.auth.leeway_seconds,
        ));
        let attempt_guard = Arc::new(AttemptGuard::new());

        Ok(Self {
            auth: auth_service,
            token_service,
            refresh_token_service,
            room_repository,
            room_lifecycle,
            password_hash,
            admin_session,
            attempt_guard,
            roles_cache,
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
        let storage = crate::storage::from_config(&config.storage)?;
        let services = Services::new(
            &config,
            db_pool,
            storage,
            std::sync::Arc::new(crate::state::RuntimeConfigOverrides::default()),
        )?;

        // 验证服务创建成功
        assert!(!services.token_service.get_secret().is_empty());

        Ok(())
    }
}
