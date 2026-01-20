//! 应用程序常量定义
//!
//! 此模块统一管理所有应用程序级别的常量，避免常量分散在各个文件中。

pub mod room {
    /// 房间默认最大内容大小 (50MB)
    pub const DEFAULT_MAX_ROOM_CONTENT_SIZE: i64 = 50 * 1024 * 1024;

    /// 房间默认最大进入次数
    pub const DEFAULT_MAX_TIMES_ENTER_ROOM: i64 = 100;

    /// 房间名称最小长度
    pub const MIN_NAME_LENGTH: usize = 3;

    /// 房间名称最大长度
    pub const MAX_NAME_LENGTH: usize = 50;
}

pub mod database {
    /// 默认数据库连接 URL
    pub const DEFAULT_DB_URL: &str = "sqlite://app.db?mode=rwc";

    /// 默认最大连接数
    pub const DEFAULT_MAX_CONNECTIONS: u32 = 20;

    /// 默认最小连接数
    pub const DEFAULT_MIN_CONNECTIONS: u32 = 5;

    /// 数据库连接获取超时时间（秒）
    pub const ACQUIRE_TIMEOUT_SECS: u64 = 30;

    /// 连接池空闲超时时间（秒）
    pub const IDLE_TIMEOUT_SECS: u64 = 600;

    /// 连接最大生命周期（秒）
    pub const MAX_LIFETIME_SECS: u64 = 1800;

    /// 数据库忙等待超时时间（秒）
    pub const BUSY_TIMEOUT_SECS: u64 = 30;
}

pub mod auth {
    /// 默认 JWT 密钥
    ///  注意：默认配置不应该在生产环境中使用
    pub const DEFAULT_JWT_SERCET: &str = "default-secret-change-in-production"; // pragma: allowlist secret

    /// 默认 JWT 过期时间（秒）- 2 小时
    pub const DEFAULT_TTL_SECONDS: i64 = 2 * 60 * 60;

    /// JWT leeway 时间（秒）- 允许的时间偏差
    pub const DEFAULT_LEEWAY_SECONDS: i64 = 5;

    /// 最大刷新令牌年龄（秒）- 30 天
    pub const MAX_REFRESH_AGE_SECONDS: i64 = 30 * 24 * 3600;

    /// 令牌即将过期的警告时间（秒）- 5 分钟
    pub const TOKEN_EXPIRY_WARNING_SECONDS: i64 = 300;
}

pub mod storage {
    /// 默认存储根目录
    pub const DEFAULT_STORAGE_ROOT: &str = "storage/rooms";
}

pub mod upload {
    /// 默认上传预留 TTL（秒）- 1 小时
    pub const DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS: i64 = 3600;

    /// 最大文件块大小
    pub const MAX_CHUNK_SIZE: usize = 1024 * 1024; // 1MB

    /// 默认 multipart 请求体上限（字节）
    pub const MAX_MULTIPART_BODY_SIZE: usize = 100 * 1024 * 1024; // 100MB

    /// 允许的文件扩展名
    pub const ALLOWED_EXTENSIONS: &[&str] = &[
        "txt", "pdf", "doc", "docx", "jpg", "jpeg", "png", "gif", "zip", "rar", "7z", "tar", "gz",
        "mp4", "avi", "mov", "mp3", "wav", "flac",
    ];

    /// 最大文件名长度
    pub const MAX_FILENAME_LENGTH: usize = 255;
}

pub mod test {
    /// 测试用上传预留 TTL（秒）
    pub const TEST_UPLOAD_RESERVATION_TTL_SECONDS: i64 = 10;

    /// 测试用 JWT 密钥
    pub const TEST_JWT_SECRET: &str = "test-secret";

    /// 测试用 JWT TTL（秒）- 2 小时
    pub const TEST_JWT_TTL_SECONDS: i64 = 120 * 60;
}

pub mod config {
    /// 默认服务器地址
    pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1";

    /// 默认服务器端口
    pub const DEFAULT_SERVER_PORT: u16 = 4092;

    /// 默认配置目录名
    pub const DEFAULT_DIR_NAME: &str = ".config";

    /// 默认应用名
    pub const DEFAULT_APP_NAME: &str = "elizabeth";

    /// 默认配置文件名
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config";

    /// 默认配置文件扩展名
    pub const DEFAULT_CONFIG_FILE_EXTENSION: &str = "yaml";

    /// 文件锁最大尝试次数
    pub const MAX_LOCK_ATTEMPTS: u8 = 3;

    /// 文件锁重试间隔（毫秒）
    pub const LOCK_RETRY_INTERVAL_MS: u64 = 100;
}

pub mod validation {
    /// 最小密码长度
    pub const MIN_PASSWORD_LENGTH: usize = 6;

    /// 最大密码长度
    pub const MAX_PASSWORD_LENGTH: usize = 128;

    /// JWT 密钥最小长度
    pub const MIN_JWT_SECRET_LENGTH: usize = 32;
}
