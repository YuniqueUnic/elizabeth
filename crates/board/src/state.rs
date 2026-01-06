/// 应用程序状态模块
///
/// 重构后的 AppState，职责更加清晰，依赖关系更加明确
use std::sync::Arc;

use anyhow::Result;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::services::Services;
use crate::websocket::{broadcaster::Broadcaster, connection::ConnectionManager};

/// 应用程序状态
///
/// 包含应用程序运行时所需的所有核心组件
#[derive(Clone)]
pub struct AppState {
    /// 数据库连接池
    pub db_pool: Arc<DbPool>,
    /// 应用程序配置
    pub config: AppConfig,
    /// 服务容器
    pub services: Services,
    /// WebSocket 连接管理器
    pub connection_manager: Arc<ConnectionManager>,
    /// WebSocket 广播器
    pub broadcaster: Arc<Broadcaster>,
}

impl AppState {
    /// 创建新的应用程序状态
    pub fn new(config: AppConfig, db_pool: Arc<DbPool>) -> Result<Self> {
        // 验证配置
        config.validate()?;

        // 创建服务
        let services = Services::new(&config, db_pool.clone())?;

        // 创建 WebSocket 连接管理器
        let connection_manager = Arc::new(ConnectionManager::new());

        // 创建 WebSocket 广播器
        let broadcaster = Arc::new(Broadcaster::new(connection_manager.clone()));

        Ok(Self {
            db_pool,
            config,
            services,
            connection_manager,
            broadcaster,
        })
    }

    /// 获取配置的引用
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// 获取服务的引用
    pub fn services(&self) -> &Services {
        &self.services
    }

    /// 获取数据库连接池的引用
    pub fn db_pool(&self) -> &Arc<DbPool> {
        &self.db_pool
    }

    /// 便捷方法：获取认证服务
    pub fn auth_service(&self) -> &crate::services::AuthService {
        &self.services.auth
    }

    /// 便捷方法：获取令牌服务
    pub fn token_service(&self) -> &crate::services::RoomTokenService {
        &self.services.token_service
    }

    /// 便捷方法：获取刷新令牌服务
    pub fn refresh_token_service(&self) -> &crate::services::RefreshTokenService {
        &self.services.refresh_token_service
    }

    /// 便捷方法：获取存储根目录
    pub fn storage_root(&self) -> &std::path::PathBuf {
        &self.config.storage.root
    }

    /// 便捷方法：获取上传预留 TTL
    pub fn upload_reservation_ttl(&self) -> chrono::Duration {
        chrono::Duration::seconds(self.config.storage.upload_reservation_ttl_seconds)
    }

    /// 便捷方法：获取房间最大内容大小
    pub fn room_max_size(&self) -> i64 {
        self.config.room.max_content_size
    }

    /// 便捷方法：获取房间最大进入次数
    pub fn room_max_times_entered(&self) -> i64 {
        self.config.room.max_times_entered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthConfig;
    use crate::db::{DbPoolSettings, init_db};

    #[tokio::test]
    async fn test_app_state_creation() -> Result<()> {
        // 创建测试数据库
        let db_settings = DbPoolSettings::new("sqlite::memory:");
        let db_pool = Arc::new(init_db(&db_settings).await?);

        // 创建测试配置
        let mut config = AppConfig::for_development();
        config.auth = AuthConfig::new("test-secret-key-for-unit-testing-123".to_string())?;

        // 创建应用状态
        let app_state = AppState::new(config, db_pool)?;

        // 验证创建成功
        assert_eq!(
            app_state.room_max_size(),
            crate::constants::room::DEFAULT_MAX_ROOM_CONTENT_SIZE
        );
        assert_eq!(
            app_state.room_max_times_entered(),
            crate::constants::room::DEFAULT_MAX_TIMES_ENTER_ROOM
        );

        Ok(())
    }
}
