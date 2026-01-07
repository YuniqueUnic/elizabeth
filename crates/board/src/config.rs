/// 应用程序配置模块
///
/// 将配置相关的设置集中管理，与 AppState 分离
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Duration;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub max_content_size: i64,
    pub max_times_entered: i64,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            max_content_size: DEFAULT_MAX_ROOM_CONTENT_SIZE,
            max_times_entered: DEFAULT_MAX_TIMES_ENTER_ROOM,
        }
    }
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub ttl_seconds: i64,
    pub leeway_seconds: i64,
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
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: DEFAULT_JWT_SERCET.into(),
            ttl_seconds: DEFAULT_TTL_SECONDS,
            leeway_seconds: DEFAULT_LEEWAY_SECONDS,
        }
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
        if self.room.max_content_size <= 0 {
            return Err(ConfigError::InvalidRoomConfig(
                "Max content size must be positive".to_string(),
            ));
        }

        if self.room.max_times_entered <= 0 {
            return Err(ConfigError::InvalidRoomConfig(
                "Max times entered must be positive".to_string(),
            ));
        }

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
