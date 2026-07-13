/// 应用程序配置模块
///
/// 将配置相关的设置集中管理，与 AppState 分离
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::models::permission::RoomPermission;

use crate::constants::{
    auth::{DEFAULT_JWT_SERCET, DEFAULT_LEEWAY_SECONDS, DEFAULT_TTL_SECONDS},
    room::{DEFAULT_MAX_ROOM_CONTENT_SIZE, DEFAULT_MAX_TIMES_ENTER_ROOM},
    storage::DEFAULT_STORAGE_ROOT,
    upload::DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
    validation::MIN_JWT_SECRET_LENGTH,
};

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub room: RoomConfig,
    pub auth: AuthConfig,
}

/// 数据库配置
#[derive(Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库连接 URL
    /// 支持：
    /// - SQLite: sqlite:path/to/db.sqlite
    /// - PostgreSQL: postgresql://user:password@host:port/database # pragma: allowlist secret
    /// - Supabase: postgresql://postgres:[PASSWORD]@db.[PROJECT].supabase.co:5432/postgres # pragma: allowlist secret
    pub url: String,

    /// 连接池最大连接数
    #[serde(default = "DatabaseConfig::default_max_connections")]
    pub max_connections: u32,

    /// 连接池最小连接数
    #[serde(default = "DatabaseConfig::default_min_connections")]
    pub min_connections: u32,

    /// SQLite journal 模式
    #[serde(default = "DatabaseConfig::default_journal_mode")]
    pub journal_mode: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:elizabeth.db".to_string(),
            max_connections: Self::default_max_connections(),
            min_connections: Self::default_min_connections(),
            journal_mode: Self::default_journal_mode(),
        }
    }
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &configrs::database_url_for_debug(&self.url))
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .field("journal_mode", &self.journal_mode)
            .finish()
    }
}

impl DatabaseConfig {
    fn default_max_connections() -> u32 {
        10
    }
    fn default_min_connections() -> u32 {
        1
    }
    fn default_journal_mode() -> String {
        "wal".to_string()
    }

    /// 检测数据库类型
    pub fn database_kind(&self) -> DatabaseKind {
        if self.url.starts_with("sqlite:") {
            DatabaseKind::Sqlite
        } else if self.url.starts_with("postgresql:") || self.url.starts_with("postgres:") {
            DatabaseKind::PostgreSQL
        } else {
            DatabaseKind::Unknown
        }
    }

    /// 检查是否为 Supabase 连接
    pub fn is_supabase(&self) -> bool {
        self.url.contains(".supabase.co")
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.url.is_empty() {
            return Err(ConfigError::InvalidDatabaseConfig(
                "Database URL cannot be empty".to_string(),
            ));
        }

        if self.max_connections == 0 {
            return Err(ConfigError::InvalidDatabaseConfig(
                "Max connections must be greater than 0".to_string(),
            ));
        }

        if self.min_connections > self.max_connections {
            return Err(ConfigError::InvalidDatabaseConfig(
                "Min connections cannot exceed max connections".to_string(),
            ));
        }

        Ok(())
    }
}

/// 数据库类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseKind {
    Sqlite,
    PostgreSQL,
    Unknown,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4092,
        }
    }
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub root: PathBuf,
    pub upload_reservation_ttl_seconds: i64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from(DEFAULT_STORAGE_ROOT),
            upload_reservation_ttl_seconds: DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS,
        }
    }
}

/// 房间配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub defaults: RoomCreationDefaults,
    pub expiry: RoomExpiryPolicy,
    pub share_disabled_lock_duration: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RoomCreationDefaults {
    pub password: Option<String>,
    pub max_times_entered: i64,
    pub max_content_size: i64,
    pub permission: RoomPermission,
}

impl TryFrom<&configrs::RoomConfig> for RoomConfig {
    type Error = ConfigError;

    fn try_from(value: &configrs::RoomConfig) -> Result<Self, Self::Error> {
        let max_content_size = i64::try_from(value.defaults.max_size.as_u64()).map_err(|_| {
            ConfigError::InvalidRoomConfig(
                "Default room max content size exceeds the supported range".to_string(),
            )
        })?;
        let share_disabled_lock_duration =
            i64::try_from(value.share_disabled_lock_duration.as_secs()).map_err(|_| {
                ConfigError::InvalidRoomConfig(
                    "Share-disabled lock duration exceeds the supported range".to_string(),
                )
            })?;
        let password = value
            .defaults
            .password
            .as_deref()
            .map(str::trim)
            .filter(|password| !password.is_empty())
            .map(str::to_owned);
        if password
            .as_ref()
            .is_some_and(|password| password.len() < 4 || password.len() > 100)
        {
            return Err(ConfigError::InvalidRoomConfig(
                "Default room password must be empty or between 4 and 100 characters".to_string(),
            ));
        }
        let mut permission = RoomPermission::empty();
        if value.defaults.permissions.read {
            permission |= RoomPermission::VIEW_ONLY;
        }
        if value.defaults.permissions.edit {
            permission |= RoomPermission::EDITABLE;
        }
        if value.defaults.permissions.share {
            permission |= RoomPermission::SHARE;
        }
        if value.defaults.permissions.delete {
            permission |= RoomPermission::DELETE;
        }
        let expiry = RoomExpiryPolicy::try_from(&value.expiry)?;

        Ok(Self {
            defaults: RoomCreationDefaults {
                password,
                max_times_entered: value.defaults.max_times_entered,
                max_content_size,
                permission,
            },
            expiry,
            share_disabled_lock_duration,
        })
    }
}

pub const DEFAULT_ROOM_ALLOWED_AGES_SECONDS: [i64; 8] = [
    60,
    30 * 60,
    2 * 60 * 60,
    12 * 60 * 60,
    24 * 60 * 60,
    7 * 24 * 60 * 60,
    30 * 24 * 60 * 60,
    365 * 24 * 60 * 60,
];
pub const DEFAULT_ROOM_AGE_SECONDS: i64 = 2 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoomExpiryPolicy {
    allowed_ages_seconds: Vec<i64>,
    default_age_seconds: i64,
}

impl RoomExpiryPolicy {
    pub fn new(
        allowed_ages_seconds: Vec<i64>,
        default_age_seconds: i64,
    ) -> Result<Self, ConfigError> {
        let policy = Self {
            allowed_ages_seconds,
            default_age_seconds,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn allowed_ages_seconds(&self) -> &[i64] {
        &self.allowed_ages_seconds
    }

    pub fn default_age_seconds(&self) -> i64 {
        self.default_age_seconds
    }

    pub fn allows(&self, age_seconds: i64) -> bool {
        self.allowed_ages_seconds
            .binary_search(&age_seconds)
            .is_ok()
    }

    pub fn expire_at(
        &self,
        from: chrono::NaiveDateTime,
        age_seconds: i64,
    ) -> Option<chrono::NaiveDateTime> {
        if !self.allows(age_seconds) {
            return None;
        }
        from.checked_add_signed(Duration::seconds(age_seconds))
    }

    pub fn default_expire_at(&self, from: chrono::NaiveDateTime) -> Option<chrono::NaiveDateTime> {
        self.expire_at(from, self.default_age_seconds)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.allowed_ages_seconds.is_empty() {
            return Err(ConfigError::InvalidRoomConfig(
                "At least one room expiry age is required".to_string(),
            ));
        }
        if self.allowed_ages_seconds.iter().any(|age| *age <= 0) {
            return Err(ConfigError::InvalidRoomConfig(
                "Room expiry ages must be positive".to_string(),
            ));
        }
        if self
            .allowed_ages_seconds
            .windows(2)
            .any(|ages| ages[0] >= ages[1])
        {
            return Err(ConfigError::InvalidRoomConfig(
                "Room expiry ages must be unique and strictly increasing".to_string(),
            ));
        }
        if !self.allows(self.default_age_seconds) {
            return Err(ConfigError::InvalidRoomConfig(
                "Default room expiry age must be included in allowed ages".to_string(),
            ));
        }
        Ok(())
    }
}

impl TryFrom<&configrs::RoomExpiryConfig> for RoomExpiryPolicy {
    type Error = ConfigError;

    fn try_from(value: &configrs::RoomExpiryConfig) -> Result<Self, Self::Error> {
        let allowed_ages_seconds = value
            .allowed_ages
            .iter()
            .map(|age| {
                i64::try_from(age.as_secs()).map_err(|_| {
                    ConfigError::InvalidRoomConfig(
                        "Room expiry age exceeds the supported range".to_string(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let default_age_seconds = i64::try_from(value.default_age.as_secs()).map_err(|_| {
            ConfigError::InvalidRoomConfig(
                "Default room expiry age exceeds the supported range".to_string(),
            )
        })?;
        Self::new(allowed_ages_seconds, default_age_seconds)
    }
}

impl Default for RoomExpiryPolicy {
    fn default() -> Self {
        Self {
            allowed_ages_seconds: DEFAULT_ROOM_ALLOWED_AGES_SECONDS.to_vec(),
            default_age_seconds: DEFAULT_ROOM_AGE_SECONDS,
        }
    }
}

impl Default for RoomCreationDefaults {
    fn default() -> Self {
        Self {
            password: None,
            max_times_entered: DEFAULT_MAX_TIMES_ENTER_ROOM,
            max_content_size: DEFAULT_MAX_ROOM_CONTENT_SIZE,
            permission: RoomPermission::new().with_all(),
        }
    }
}

impl fmt::Debug for RoomCreationDefaults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RoomCreationDefaults")
            .field(
                "password",
                &configrs::optional_secret_for_debug(self.password.as_deref()),
            )
            .field("max_times_entered", &self.max_times_entered)
            .field("max_content_size", &self.max_content_size)
            .field("permission", &self.permission)
            .finish()
    }
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            defaults: RoomCreationDefaults::default(),
            expiry: RoomExpiryPolicy::default(),
            share_disabled_lock_duration: 3600,
        }
    }
}

/// 认证配置
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub ttl_seconds: i64,
    pub leeway_seconds: i64,
    pub refresh_ttl_seconds: i64,
    pub cleanup_interval_seconds: u64,
    pub enable_refresh_token_rotation: bool,
}

impl AuthConfig {
    pub fn new(jwt_secret: String) -> Result<Self, ConfigError> {
        if jwt_secret.len() < MIN_JWT_SECRET_LENGTH {
            return Err(ConfigError::InvalidJwtSecret(format!(
                "JWT secret must be at least {} characters",
                MIN_JWT_SECRET_LENGTH
            )));
        }

        Ok(Self {
            jwt_secret,
            ttl_seconds: DEFAULT_TTL_SECONDS,
            leeway_seconds: DEFAULT_LEEWAY_SECONDS,
            refresh_ttl_seconds: 7 * 24 * 60 * 60,
            cleanup_interval_seconds: 24 * 60 * 60,
            enable_refresh_token_rotation: true,
        })
    }

    pub fn with_ttl(mut self, ttl_seconds: i64) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }

    pub fn with_leeway(mut self, leeway_seconds: i64) -> Self {
        self.leeway_seconds = leeway_seconds;
        self
    }

    pub fn with_refresh_policy(
        mut self,
        refresh_ttl_seconds: i64,
        cleanup_interval_seconds: i64,
        enable_rotation: bool,
    ) -> Self {
        self.refresh_ttl_seconds = refresh_ttl_seconds.max(1);
        self.cleanup_interval_seconds = cleanup_interval_seconds.max(1) as u64;
        self.enable_refresh_token_rotation = enable_rotation;
        self
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: DEFAULT_JWT_SERCET.into(),
            ttl_seconds: DEFAULT_TTL_SECONDS,
            leeway_seconds: DEFAULT_LEEWAY_SECONDS,
            refresh_ttl_seconds: 7 * 24 * 60 * 60,
            cleanup_interval_seconds: 24 * 60 * 60,
            enable_refresh_token_rotation: true,
        }
    }
}

impl fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthConfig")
            .field("jwt_secret", &configrs::secret_for_debug(&self.jwt_secret))
            .field("ttl_seconds", &self.ttl_seconds)
            .field("leeway_seconds", &self.leeway_seconds)
            .field("refresh_ttl_seconds", &self.refresh_ttl_seconds)
            .field("cleanup_interval_seconds", &self.cleanup_interval_seconds)
            .field(
                "enable_refresh_token_rotation",
                &self.enable_refresh_token_rotation,
            )
            .finish()
    }
}

/// 配置错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid JWT secret: {0}")]
    InvalidJwtSecret(String),
    #[error("Invalid storage path: {0}")]
    InvalidStoragePath(String),
    #[error("Invalid room configuration: {0}")]
    InvalidRoomConfig(String),
    #[error("Invalid database configuration: {0}")]
    InvalidDatabaseConfig(String),
}

/// 配置验证器
impl AppConfig {
    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 验证数据库配置
        self.database.validate()?;

        // 验证存储路径
        if self.storage.root.as_os_str().is_empty() {
            return Err(ConfigError::InvalidStoragePath(
                "Storage root path cannot be empty".to_string(),
            ));
        }

        // 验证房间配置
        if self.room.defaults.max_content_size <= 0 {
            return Err(ConfigError::InvalidRoomConfig(
                "Default room max content size must be positive".to_string(),
            ));
        }

        if self.room.defaults.max_times_entered <= 0 {
            return Err(ConfigError::InvalidRoomConfig(
                "Default room max times entered must be positive".to_string(),
            ));
        }

        if self.room.share_disabled_lock_duration <= 0 {
            return Err(ConfigError::InvalidRoomConfig(
                "Share-disabled lock duration must be positive".to_string(),
            ));
        }

        self.room.expiry.validate()?;

        Ok(())
    }

    /// 创建生产环境配置
    pub fn for_production(jwt_secret: String) -> Result<Self, ConfigError> {
        let auth_config = AuthConfig::new(jwt_secret)?;

        Ok(Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            storage: StorageConfig::default(),
            room: RoomConfig::default(),
            auth: auth_config,
        })
    }

    /// 创建开发环境配置
    pub fn for_development() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            storage: StorageConfig::default(),
            room: RoomConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_jwt_secret() {
        let result = AuthConfig::new("short".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_jwt_secret() {
        let result = AuthConfig::new("this-is-a-valid-secret-key-for-testing".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = AppConfig::for_development();
        assert!(config.validate().is_ok());
    }
}
