/// 应用程序状态模块
///
/// 重构后的 AppState，职责更加清晰，依赖关系更加明确
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use anyhow::Result;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::services::Services;
use crate::storage::{StorageBackend, from_config};
use crate::websocket::{broadcaster::Broadcaster, connection::ConnectionManager};

/// 平台管理 API 凭证的环境变量名。
pub const ADMIN_TOKEN_ENV: &str = "ELIZABETH_ADMIN_TOKEN";

/// 运行时可写配置覆盖（管理白名单）。
/// 仅存活于进程内，重启后回到配置文件值——configrs 始终是唯一配置源。
/// 经 Arc 共享：所有 AppState 克隆看到同一份覆盖。
#[derive(Debug)]
pub struct RuntimeConfigOverrides {
    /// true = 对搜索引擎关闭索引（robots.txt Disallow + X-Robots-Tag: noindex）
    disallow_search_indexing: AtomicBool,
    /// 0 = 未覆盖，回退配置文件默认值
    room_default_max_size: AtomicI64,
    /// 0 = 未覆盖，回退配置文件默认值
    room_default_max_times_entered: AtomicI64,
    /// 0 = 未覆盖，回退配置文件默认值（访问令牌有效期，秒）
    session_ttl_seconds: AtomicI64,
    /// 0 = 未覆盖，回退配置文件默认值（上传预留有效期，秒）
    upload_reservation_ttl_seconds: AtomicI64,
    /// None = 未覆盖，回退配置文件默认值（新房间默认加入角色）
    room_default_role_key: RwLock<Option<String>>,
    /// None = 未覆盖，回退配置文件默认值（房间有效期策略，整组替换）
    room_expiry_policy: RwLock<Option<Arc<crate::config::RoomExpiryPolicy>>>,
}

impl Default for RuntimeConfigOverrides {
    /// 安全默认：搜索引擎防索引开启（与 configrs 安全配置默认一致）。
    fn default() -> Self {
        Self {
            disallow_search_indexing: AtomicBool::new(true),
            room_default_max_size: AtomicI64::new(0),
            room_default_max_times_entered: AtomicI64::new(0),
            session_ttl_seconds: AtomicI64::new(0),
            upload_reservation_ttl_seconds: AtomicI64::new(0),
            room_default_role_key: RwLock::new(None),
            room_expiry_policy: RwLock::new(None),
        }
    }
}

impl RuntimeConfigOverrides {
    pub fn disallow_search_indexing(&self) -> bool {
        self.disallow_search_indexing.load(Ordering::Relaxed)
    }

    pub fn set_disallow_search_indexing(&self, value: bool) {
        self.disallow_search_indexing
            .store(value, Ordering::Relaxed);
    }

    /// 房间默认容量；0 表示未覆盖。
    pub fn room_default_max_size(&self) -> i64 {
        self.room_default_max_size.load(Ordering::Relaxed)
    }

    pub fn set_room_default_max_size(&self, value: i64) {
        self.room_default_max_size.store(value, Ordering::Relaxed);
    }

    /// 房间默认进入次数；0 表示未覆盖。
    pub fn room_default_max_times_entered(&self) -> i64 {
        self.room_default_max_times_entered.load(Ordering::Relaxed)
    }

    pub fn set_room_default_max_times_entered(&self, value: i64) {
        self.room_default_max_times_entered
            .store(value, Ordering::Relaxed);
    }

    /// 访问令牌有效期（秒）；0 表示未覆盖。
    pub fn session_ttl_seconds(&self) -> i64 {
        self.session_ttl_seconds.load(Ordering::Relaxed)
    }

    pub fn set_session_ttl_seconds(&self, value: i64) {
        self.session_ttl_seconds.store(value, Ordering::Relaxed);
    }

    /// 上传预留有效期（秒）；0 表示未覆盖。
    pub fn upload_reservation_ttl_seconds(&self) -> i64 {
        self.upload_reservation_ttl_seconds.load(Ordering::Relaxed)
    }

    pub fn set_upload_reservation_ttl_seconds(&self, value: i64) {
        self.upload_reservation_ttl_seconds
            .store(value, Ordering::Relaxed);
    }

    /// 新房间默认加入角色；None 表示未覆盖。
    pub fn room_default_role_key(&self) -> Option<String> {
        self.room_default_role_key
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    pub fn set_room_default_role_key(&self, value: Option<String>) {
        if let Ok(mut guard) = self.room_default_role_key.write() {
            *guard = value;
        }
    }

    /// 房间有效期策略覆盖；None 表示未覆盖。
    pub fn room_expiry_policy(&self) -> Option<Arc<crate::config::RoomExpiryPolicy>> {
        self.room_expiry_policy
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    pub fn set_room_expiry_policy(&self, value: Option<Arc<crate::config::RoomExpiryPolicy>>) {
        if let Ok(mut guard) = self.room_expiry_policy.write() {
            *guard = value;
        }
    }
}

/// 平台管理 API 凭证：环境变量为引导值，支持运行时轮换（进程内覆盖，重启回退环境值）。
#[derive(Debug, Default)]
pub struct AdminCredential {
    runtime_token: RwLock<Option<String>>,
}

impl AdminCredential {
    /// 轮换管理凭证；空串或纯空白视为无效，调用方负责先校验强度。
    pub fn rotate(&self, token: String) {
        if let Ok(mut guard) = self.runtime_token.write() {
            *guard = Some(token);
        }
    }

    /// 当前凭证来源：运行时覆盖或环境变量引导值。
    pub fn source(&self) -> &'static str {
        let overridden = self
            .runtime_token
            .read()
            .ok()
            .is_some_and(|guard| guard.is_some());
        if overridden {
            "runtime-override"
        } else {
            "env"
        }
    }

    /// 校验请求携带的管理凭证。未配置任何凭证时整个管理 API 关闭。
    pub fn verify(&self, provided: Option<&str>) -> Result<(), crate::errors::AppError> {
        use crate::errors::AppError;

        let expected = self
            .runtime_token
            .read()
            .ok()
            .and_then(|guard| guard.clone())
            .or_else(|| {
                std::env::var(ADMIN_TOKEN_ENV)
                    .ok()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty())
            });

        match expected {
            None => Err(AppError::authorization(format!(
                "Admin API disabled (set {ADMIN_TOKEN_ENV})"
            ))),
            Some(expected) if provided == Some(expected.as_str()) => Ok(()),
            _ => Err(AppError::authorization("Invalid admin token")),
        }
    }
}

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
    /// 内容存储后端
    pub storage: Arc<dyn StorageBackend>,
    /// 运行时可写配置覆盖（进程内，重启回退到配置文件）
    pub runtime: Arc<RuntimeConfigOverrides>,
    /// 平台管理 API 凭证（环境引导 + 运行时轮换）
    pub admin_credential: Arc<AdminCredential>,
    /// WebSocket 连接管理器
    pub connection_manager: Arc<ConnectionManager>,
    /// WebSocket 广播器
    pub broadcaster: Arc<Broadcaster>,
    /// 房间角色矩阵缓存（键：room_id，失效：rooms.roles_version）
    pub roles_cache: Arc<crate::authz::RoleTableCache>,
    /// 配置文件中的房间有效期策略；运行时覆盖存在时以其为准
    base_room_expiry_policy: Arc<crate::config::RoomExpiryPolicy>,
}

impl AppState {
    /// 创建新的应用程序状态
    pub fn new(config: AppConfig, db_pool: Arc<DbPool>) -> Result<Self> {
        // 验证配置
        config.validate()?;

        // 按配置选择内容存储后端
        let storage = from_config(&config.storage)?;

        Self::with_storage(
            config,
            db_pool,
            storage,
            Arc::new(RuntimeConfigOverrides::default()),
        )
    }

    /// 用显式指定的存储后端创建应用状态（测试注入用）。
    pub fn with_storage(
        config: AppConfig,
        db_pool: Arc<DbPool>,
        storage: Arc<dyn StorageBackend>,
        runtime: Arc<RuntimeConfigOverrides>,
    ) -> Result<Self> {
        config.validate()?;

        // 创建服务
        let services = Services::new(&config, db_pool.clone(), storage.clone(), runtime.clone())?;

        // 创建 WebSocket 连接管理器
        let connection_manager = Arc::new(ConnectionManager::new());

        // 创建 WebSocket 广播器
        let broadcaster = Arc::new(Broadcaster::new(connection_manager.clone()));

        let roles_cache = services.roles_cache.clone();
        let base_room_expiry_policy = Arc::new(config.room.expiry.clone());
        Ok(Self {
            db_pool,
            config,
            services,
            storage,
            runtime,
            admin_credential: Arc::new(AdminCredential::default()),
            connection_manager,
            broadcaster,
            roles_cache,
            base_room_expiry_policy,
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

    pub fn room_password_service(&self) -> &crate::services::RoomPasswordService {
        &self.services.room_password
    }

    pub fn attempt_guard(&self) -> &crate::services::AttemptGuard {
        &self.services.attempt_guard
    }

    /// 便捷方法：获取存储根目录
    pub fn storage_root(&self) -> &std::path::PathBuf {
        &self.config.storage.root
    }

    /// 全局内容去重是否开启（启动期配置）
    pub fn global_dedup_enabled(&self) -> bool {
        self.config.storage.global_dedup
    }

    /// 便捷方法：获取上传预留 TTL（运行时白名单覆盖优先）。
    pub fn upload_reservation_ttl(&self) -> chrono::Duration {
        let seconds = match self.runtime.upload_reservation_ttl_seconds() {
            0 => self.config.storage.upload_reservation_ttl_seconds,
            secs => secs,
        };
        chrono::Duration::seconds(seconds)
    }

    /// 访问令牌签发 TTL（运行时白名单覆盖优先，回退配置文件值）。
    pub fn session_ttl(&self) -> chrono::Duration {
        match self.runtime.session_ttl_seconds() {
            0 => self.services.token_service.get_ttl(),
            secs => chrono::Duration::seconds(secs),
        }
    }

    /// 内容传输模式
    pub fn transfer_mode(&self) -> crate::config::TransferMode {
        self.config.storage.transfer
    }

    /// 预签名 URL 有效期
    pub fn presign_ttl(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.storage.presign_ttl_seconds.max(1) as u64)
    }

    pub fn room_creation_defaults(&self) -> &crate::config::RoomCreationDefaults {
        &self.config.room.defaults
    }

    /// 房间有效期策略：运行时白名单覆盖优先，回退配置文件值。
    /// 覆盖值在写入边界已校验，因此这里不会构造出非法策略。
    pub fn room_expiry_policy(&self) -> Arc<crate::config::RoomExpiryPolicy> {
        self.runtime
            .room_expiry_policy()
            .unwrap_or_else(|| self.base_room_expiry_policy.clone())
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
            app_state.room_creation_defaults().max_content_size,
            crate::constants::room::DEFAULT_MAX_ROOM_CONTENT_SIZE
        );
        assert_eq!(
            app_state.room_creation_defaults().max_times_entered,
            crate::constants::room::DEFAULT_MAX_TIMES_ENTER_ROOM
        );

        Ok(())
    }
}
