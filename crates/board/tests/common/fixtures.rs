use board::models::permission::RoomPermission;
use board::models::{Room, RoomToken};
use board::services::token::RoomTokenClaims;
/// 测试数据夹具
///
/// 提供常用的测试数据对象
use chrono::{NaiveDateTime, Utc};

/// 创建测试房间
pub fn create_test_room(name: &str, password: Option<String>) -> Room {
    Room::new(name.to_string(), password)
}

/// 创建带密码的测试房间
pub fn create_test_room_with_password(name: &str, password: &str) -> Room {
    Room::new(name.to_string(), Some(password.to_string()))
}

/// 创建测试房间令牌记录
pub fn create_test_room_token(room_id: i64, jti: &str) -> RoomToken {
    RoomToken::new(room_id, jti.to_string(), Utc::now().naive_utc())
}

/// 创建测试令牌声明
pub fn create_test_token_claims(
    room_id: i64,
    room_name: &str,
    permission: RoomPermission,
) -> RoomTokenClaims {
    RoomTokenClaims {
        sub: room_name.to_string(),
        room_id,
        room_name: room_name.to_string(),
        permission: permission.bits(),
        max_size: 1024 * 1024,              // 1MB
        exp: Utc::now().timestamp() + 3600, // 1 小时后过期
        iat: Utc::now().timestamp(),
        jti: format!("test-jti-{}", Utc::now().timestamp()),
        refresh_jti: None,
        token_type: board::services::token::TokenType::Access,
    }
}

/// 创建即将过期的令牌声明
pub fn create_expiring_token_claims(
    room_id: i64,
    room_name: &str,
    permission: RoomPermission,
    minutes_to_expiry: i64,
) -> RoomTokenClaims {
    let now = Utc::now();
    RoomTokenClaims {
        sub: room_name.to_string(),
        room_id,
        room_name: room_name.to_string(),
        permission: permission.bits(),
        max_size: 1024 * 1024, // 1MB
        exp: now.timestamp() + minutes_to_expiry * 60,
        iat: now.timestamp(),
        jti: format!("test-jti-{}", now.timestamp()),
        refresh_jti: None,
        token_type: board::services::token::TokenType::Access,
    }
}

/// 常用的测试房间名称
pub mod room_names {
    pub const TEST_ROOM: &str = "test-room";
    pub const PRIVATE_ROOM: &str = "private-room";
    pub const PUBLIC_ROOM: &str = "public-room";
    pub const EMPTY_ROOM: &str = "";
}

/// 常用的测试密码
pub mod passwords {
    pub const SIMPLE: &str = "password123";
    pub const COMPLEX: &str = "ComplexP@ssw0rd!";
    pub const EMPTY: &str = "";
    pub const NUMERIC: &str = "123456";
}

/// 常用的文件名用于上传测试
pub mod filenames {
    pub const TEXT_FILE: &str = "test.txt";
    pub const IMAGE_FILE: &str = "image.jpg";
    pub const PDF_FILE: &str = "document.pdf";
    pub const LARGE_FILE: &str = "large-file.zip";
}

/// 文件大小常量（字节）
pub mod file_sizes {
    pub const SMALL: usize = 1024; // 1KB
    pub const MEDIUM: usize = 10 * 1024; // 10KB
    pub const LARGE: usize = 1024 * 1024; // 1MB
    pub const HUGE: usize = 10 * 1024 * 1024; // 10MB
}
