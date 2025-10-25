use std::default;

use smart_default::SmartDefault;

use crate::merge::{Merge, overwrite, overwrite_not_empty_string};

#[derive(Merge, Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub jwt: JwtConfig,
    pub room: RoomConfig,
    pub upload: UploadConfig,
    pub middleware: MiddlewareConfig,
}

#[derive(Merge, Debug, Clone, SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ServerConfig {
    #[default("127.0.0.1")]
    #[merge(strategy = overwrite_not_empty_string)]
    pub addr: String,
    #[default(4092)]
    #[merge(strategy = overwrite)]
    pub port: u16,
}

#[derive(Merge, Debug, Clone, SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LoggingConfig {
    #[default("info")]
    #[merge(strategy = overwrite_not_empty_string)]
    pub level: String,
}

#[derive(Merge, Debug, Clone, SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DatabaseConfig {
    #[default("sqlite:app.db")]
    #[merge(strategy = overwrite_not_empty_string)]
    pub url: String,
    #[default(Some(20))]
    #[merge(strategy = overwrite)]
    pub max_connections: Option<u32>,
    #[default(Some(5))]
    #[merge(strategy = overwrite)]
    pub min_connections: Option<u32>,
}

#[derive(Merge, Debug, Clone, SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct JwtConfig {
    #[default("secret")]
    #[merge(strategy = overwrite_not_empty_string)]
    pub secret: String,
    #[default(30 * 60)]
    #[merge(strategy = overwrite)]
    pub ttl_seconds: i64,
    #[default(5)]
    #[merge(strategy = overwrite)]
    pub leeway_seconds: i64,
    // 刷新令牌相关配置
    #[default(7 * 24 * 60 * 60)] // 7 天（秒）
    #[merge(strategy = overwrite)]
    pub refresh_ttl_seconds: i64,
    #[default(10)]
    #[merge(strategy = overwrite)]
    pub max_refresh_count: i64,
    #[default(24 * 60 * 60)] // 24 小时（秒）
    #[merge(strategy = overwrite)]
    pub cleanup_interval_seconds: i64,
    #[default(true)]
    #[merge(strategy = overwrite)]
    pub enable_refresh_token_rotation: bool,
}

#[derive(Merge, Debug, Clone, SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct StorageConfig {
    #[default("storage/rooms")]
    #[merge(strategy = overwrite_not_empty_string)]
    pub root: String, // pragma: allowlist secret
}

#[derive(Merge, Debug, Clone, SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct RoomConfig {
    #[default(10 * 1024 * 1024)]
    #[merge(strategy = overwrite)]
    pub max_size: i64,
    #[default(100)]
    #[merge(strategy = overwrite)]
    pub max_times_entered: i64,
}

#[derive(Merge, Debug, Clone, SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct UploadConfig {
    #[default(10)]
    #[merge(strategy = overwrite)]
    pub reservation_ttl_seconds: i64,
}

// Middleware configurations - simplified without Merge trait
#[derive(Debug, Clone, Default, Merge, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct MiddlewareConfig {
    pub tracing: TracingConfig,
    pub request_id: RequestIdConfig,
    pub compression: CompressionConfig,
    pub cors: CorsConfig,
    pub security: SecurityConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, SmartDefault, Merge, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TracingConfig {
    #[default(true)]
    #[merge(strategy = overwrite)]
    pub enabled: bool,
    #[default = "info"]
    #[merge(strategy = overwrite)]
    pub level: String,
    #[default(false)]
    #[merge(strategy = overwrite)]
    pub include_headers: bool,
    #[default(false)]
    #[merge(strategy = overwrite)]
    pub include_body: bool,
}

#[derive(Debug, Clone, SmartDefault, Merge, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct RequestIdConfig {
    #[default(true)]
    #[merge(strategy = overwrite)]
    pub enabled: bool,
    #[merge(strategy = overwrite_not_empty_string)]
    #[default = "X-Request-Id"]
    pub header_name: String,
    #[default(true)]
    #[merge(strategy = overwrite)]
    pub generate_if_missing: bool,
}

#[derive(Debug, Clone, SmartDefault, Merge, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct CompressionConfig {
    #[default(false)]
    #[merge(strategy = overwrite)]
    pub enabled: bool,
    #[default(1024)]
    #[merge(strategy = overwrite)]
    pub min_content_length: usize,
}

#[derive(Debug, Clone, SmartDefault, Merge, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct CorsConfig {
    #[default(false)]
    #[merge(strategy = overwrite)]
    pub enabled: bool,

    #[default(vec!["*".to_string()])]
    #[merge(strategy = overwrite)]
    pub allowed_origins: Vec<String>,

    #[default(vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string(), "OPTIONS".to_string()])]
    #[merge(strategy = overwrite)]
    pub allowed_methods: Vec<String>,

    #[merge(strategy = overwrite)]
    #[default(vec!["*".to_string()])]
    pub allowed_headers: Vec<String>,

    #[default(false)]
    #[merge(strategy = overwrite)]
    pub allow_credentials: bool,

    #[default(3600)]
    #[merge(strategy = overwrite)]
    pub max_age: u64,

    #[default(vec![])]
    #[merge(strategy = overwrite)]
    pub expose_headers: Vec<String>,
}

#[derive(Debug, Clone, SmartDefault, Merge, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SecurityConfig {
    #[default(true)]
    #[merge(strategy = overwrite)]
    pub enabled: bool,

    #[default(true)]
    #[merge(strategy = overwrite)]
    pub content_type_options: bool,

    #[default = "DENY"]
    #[merge(strategy = overwrite_not_empty_string)]
    pub frame_options: String,

    #[default = "1; mode=block"]
    #[merge(strategy = overwrite_not_empty_string)]
    pub xss_protection: String,

    #[default = "max-age=31536000; includeSubDomains"]
    #[merge(strategy = overwrite_not_empty_string)]
    pub strict_transport_security: String,

    #[default = "strict-origin-when-cross-origin"]
    #[merge(strategy = overwrite_not_empty_string)]
    pub referrer_policy: String,
}

#[derive(Debug, Clone, SmartDefault, Merge, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct RateLimitConfig {
    #[default(false)]
    #[merge(strategy = overwrite)]
    pub enabled: bool,
    #[default(10)]
    #[merge(strategy = overwrite)]
    pub per_second: u64,
    #[default(20)]
    #[merge(strategy = overwrite)]
    pub burst_size: u64,
    #[default(60)]
    #[merge(strategy = overwrite)]
    pub cleanup_interval_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.server.addr, "127.0.0.1");
        assert_eq!(cfg.server.port, 4092);
        assert_eq!(cfg.logging.level.to_lowercase(), "info");
        assert_eq!(cfg.database.url, "sqlite:app.db");
        assert_eq!(cfg.database.max_connections, Some(20));
        assert_eq!(cfg.database.min_connections, Some(5));
        assert_eq!(cfg.jwt.secret, "secret");
        assert_eq!(cfg.jwt.ttl_seconds, 30 * 60);
        assert_eq!(cfg.jwt.refresh_ttl_seconds, 7 * 24 * 60 * 60);
        assert_eq!(cfg.jwt.max_refresh_count, 10);
        assert_eq!(cfg.jwt.cleanup_interval_seconds, 24 * 60 * 60);
        assert_eq!(cfg.jwt.enable_refresh_token_rotation, true);
        assert_eq!(cfg.storage.root, "storage/rooms");
        assert_eq!(cfg.room.max_size, 10 * 1024 * 1024);
        assert_eq!(cfg.room.max_times_entered, 100);
        assert_eq!(cfg.upload.reservation_ttl_seconds, 10);

        // Test middleware defaults
        assert_eq!(cfg.middleware.tracing.enabled, true);
        assert_eq!(cfg.middleware.tracing.level, "info");
        assert_eq!(cfg.middleware.request_id.enabled, true);
        assert_eq!(cfg.middleware.request_id.header_name, "X-Request-Id");
        assert_eq!(cfg.middleware.compression.enabled, false);
        assert_eq!(cfg.middleware.cors.enabled, false);
        assert_eq!(cfg.middleware.security.enabled, true);
        assert_eq!(cfg.middleware.rate_limit.enabled, false);
    }

    #[test]
    fn test_merge_overwrites_non_empty() {
        let mut left = AppConfig::default();
        let right = AppConfig {
            server: ServerConfig {
                addr: "0.0.0.0".into(),
                port: 8080,
            },
            logging: LoggingConfig {
                level: "debug".into(),
            },
            database: DatabaseConfig {
                url: "sqlite://test.db".into(),
                max_connections: Some(50),
                min_connections: None,
            },
            jwt: JwtConfig {
                secret: "foobar".into(), // pragma: allowlist secret
                ttl_seconds: 120,
                leeway_seconds: 2,
                ..Default::default()
            },
            storage: StorageConfig {
                root: "/tmp/storage".into(),
            },
            room: RoomConfig {
                max_size: 42,
                max_times_entered: 7,
            },
            upload: UploadConfig {
                reservation_ttl_seconds: 30,
            },
            middleware: MiddlewareConfig::default(),
        };

        left.merge(right);

        assert_eq!(left.server.addr, "0.0.0.0");
        assert_eq!(left.server.port, 8080);
        assert_eq!(left.logging.level, "debug");
        assert_eq!(left.database.url, "sqlite://test.db");
        assert_eq!(left.database.max_connections, Some(50));
        assert_eq!(left.database.min_connections, None);
        assert_eq!(left.jwt.secret, "foobar"); // pragma: allowlist secret
        assert_eq!(left.jwt.ttl_seconds, 120);
        assert_eq!(left.jwt.leeway_seconds, 2);
        assert_eq!(left.storage.root, "/tmp/storage");
        assert_eq!(left.room.max_size, 42);
        assert_eq!(left.room.max_times_entered, 7);
        assert_eq!(left.upload.reservation_ttl_seconds, 30);
    }

    #[test]
    fn test_merge_preserves_empty_strings() {
        let mut left = AppConfig::default();
        let right = AppConfig {
            server: ServerConfig {
                addr: "".into(),
                port: 1234,
            },
            logging: LoggingConfig { level: "".into() },
            ..AppConfig::default()
        };

        left.merge(right);

        assert_eq!(left.server.addr, "127.0.0.1");
        assert_eq!(left.logging.level, "info");
        assert_eq!(left.server.port, 1234);
    }
}
