use regex::Regex;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

use crate::errors::{AppError, AppResult};

// 验证工具模块
// 提供统一的输入验证和安全检查功能

/// 房间名称验证器
pub struct RoomNameValidator;

impl RoomNameValidator {
    /// 验证房间名称
    /// 规则：
    /// - 长度：3-50 字符
    /// - 只能包含字母、数字、下划线和连字符
    /// - 不能以下划线或连字符开头/结尾
    /// - 不能连续使用下划线或连字符
    pub fn validate(name: &str) -> AppResult<()> {
        if name.len() < 3 || name.len() > 50 {
            return Err(AppError::validation(
                "Room name must be between 3 and 50 characters",
            ));
        }

        // 使用正则表达式验证房间名称格式
        let re = Regex::new(r"^[a-zA-Z0-9](?:[a-zA-Z0-9_-]{1,48}[a-zA-Z0-9])?$").unwrap();
        if !re.is_match(name) {
            return Err(AppError::validation(
                "Room name can only contain letters, numbers, underscores, and hyphens, and cannot start or end with underscore or hyphen",
            ));
        }

        Ok(())
    }
}

/// 密码验证器
pub struct PasswordValidator;

impl PasswordValidator {
    /// 验证密码强度
    /// 规则：
    /// - 长度：至少 8 字符
    /// - 包含至少一个字母
    /// - 包含至少一个数字
    /// - 可选：包含特殊字符以提高安全性
    pub fn validate(password: &str) -> AppResult<()> {
        if password.len() < 8 {
            return Err(AppError::validation(
                "Password must be at least 8 characters long",
            ));
        }

        let has_letter = password.chars().any(|c| c.is_alphabetic());
        let has_digit = password.chars().any(|c| c.is_numeric());

        if !has_letter {
            return Err(AppError::validation(
                "Password must contain at least one letter",
            ));
        }

        if !has_digit {
            return Err(AppError::validation(
                "Password must contain at least one number",
            ));
        }

        Ok(())
    }

    /// 验证房间密码（较宽松的规则）
    pub fn validate_room_password(password: &str) -> AppResult<()> {
        if password.is_empty() {
            return Ok(()); // 空密码表示无密码房间
        }

        if password.len() < 4 || password.len() > 100 {
            return Err(AppError::validation(
                "Room password must be between 4 and 100 characters",
            ));
        }

        Ok(())
    }
}

/// 文件名验证器
pub struct FilenameValidator;

impl FilenameValidator {
    /// 验证文件名安全性
    /// 规则：
    /// - 不能包含路径遍历字符 (.., /, \)
    /// - 不能包含控制字符
    /// - 长度限制：1-255 字符
    /// - 不能是保留名称 (Windows)
    pub fn validate(filename: &str) -> AppResult<()> {
        if filename.is_empty() || filename.len() > 255 {
            return Err(AppError::validation(
                "Filename must be between 1 and 255 characters",
            ));
        }

        // 检查路径遍历攻击
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Err(AppError::validation(
                "Filename cannot contain path traversal characters",
            ));
        }

        // 检查控制字符
        if filename.chars().any(|c| c.is_control()) {
            return Err(AppError::validation(
                "Filename cannot contain control characters",
            ));
        }

        // Windows 保留名称检查
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        let name_upper = filename.to_uppercase();
        if reserved_names
            .iter()
            .any(|&reserved| name_upper.starts_with(reserved))
        {
            return Err(AppError::validation("Filename cannot be a reserved name"));
        }

        // 检查特殊字符（允许的字符：字母、数字、点、下划线、连字符、空格）
        let allowed_re = Regex::new(r"^[a-zA-Z0-9._\-\s]+$").unwrap();
        if !allowed_re.is_match(filename) {
            return Err(AppError::validation(
                "Filename can only contain letters, numbers, dots, underscores, hyphens, and spaces",
            ));
        }

        Ok(())
    }

    /// 清理文件名，移除不安全字符
    pub fn sanitize(filename: &str) -> String {
        filename
            .chars()
            .map(|c| match c {
                '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ if c.is_control() => '_',
                _ => c,
            })
            .collect::<String>()
            .trim_matches(|c: char| c == '.' || c.is_whitespace())
            .to_string()
    }
}

/// JWT 令牌验证器
pub struct TokenValidator;

impl TokenValidator {
    /// 验证 JWT 令牌格式
    pub fn validate_token_format(token: &str) -> AppResult<()> {
        if token.is_empty() {
            return Err(AppError::token("Token cannot be empty"));
        }

        // 简单的 JWT 格式验证（header.payload.signature）
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AppError::token("Invalid token format"));
        }

        for part in parts {
            if part.is_empty() {
                return Err(AppError::token("Token part cannot be empty"));
            }

            // 验证 base64url 格式（简单检查）
            if !part
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Err(AppError::token("Invalid token encoding"));
            }
        }

        Ok(())
    }

    /// 从授权头中提取令牌
    pub fn extract_from_auth_header(header: &str) -> AppResult<String> {
        if !header.to_lowercase().starts_with("bearer ") {
            return Err(AppError::authentication(
                "Invalid authorization header format",
            ));
        }

        let token = header[7..].trim();
        if token.is_empty() {
            return Err(AppError::authentication(
                "Empty token in authorization header",
            ));
        }

        Ok(token.to_string())
    }
}

/// 分块上传验证器
pub struct ChunkedUploadValidator;

impl ChunkedUploadValidator {
    /// 验证分块上传参数
    pub fn validate_chunk_params(
        chunk_number: u32,
        total_chunks: u32,
        chunk_size: usize,
        file_size: u64,
    ) -> AppResult<()> {
        // 验证分块号
        if chunk_number == 0 || chunk_number > total_chunks {
            return Err(AppError::validation("Invalid chunk number"));
        }

        // 验证总分块数
        if total_chunks == 0 || total_chunks > 10000 {
            return Err(AppError::validation("Invalid total chunks number"));
        }

        // 验证分块大小
        if chunk_size == 0 || chunk_size > 10 * 1024 * 1024 {
            return Err(AppError::validation(
                "Chunk size must be between 1 and 10MB",
            ));
        }

        // 验证文件大小
        if file_size == 0 || file_size > 10 * 1024 * 1024 * 1024 {
            return Err(AppError::validation("File size must be between 1 and 10GB"));
        }

        Ok(())
    }
}

/// 内容类型验证器
pub struct ContentTypeValidator;

impl ContentTypeValidator {
    /// 验证文件内容类型
    pub fn validate_content_type(content_type: &str, allowed_types: &[&str]) -> AppResult<()> {
        if content_type.is_empty() {
            return Err(AppError::validation("Content type cannot be empty"));
        }

        // 检查是否在允许的类型列表中
        if !allowed_types.contains(&content_type) {
            return Err(AppError::unsupported_media_type(content_type));
        }

        Ok(())
    }

    /// 从 MIME 类型猜测文件扩展名
    pub fn guess_extension_from_mime(mime: &str) -> Option<&'static str> {
        match mime.to_lowercase().as_str() {
            "image/jpeg" => Some("jpg"),
            "image/png" => Some("png"),
            "image/gif" => Some("gif"),
            "application/pdf" => Some("pdf"),
            "text/plain" => Some("txt"),
            "application/json" => Some("json"),
            "application/xml" => Some("xml"),
            "application/zip" => Some("zip"),
            _ => None,
        }
    }
}

/// 验证中间件
pub struct ValidationMiddleware;

impl ValidationMiddleware {
    /// 验证房间是否存在且可访问
    pub async fn validate_room_access(
        room_id: i64,
        user_permissions: crate::models::room::permission::RoomPermission,
    ) -> AppResult<()> {
        // 这里应该查询数据库验证房间状态
        // 暂时返回成功，实际实现需要数据库查询
        Ok(())
    }

    /// 验证用户权限
    pub fn validate_user_permission(
        required: crate::models::room::permission::RoomPermission,
        user_has: crate::models::room::permission::RoomPermission,
    ) -> AppResult<()> {
        if !user_has.contains(required) {
            return Err(AppError::permission_denied(
                "Insufficient permissions for this operation",
            ));
        }
        Ok(())
    }
}

/// 安全检查工具
pub struct SecurityChecker;

impl SecurityChecker {
    /// 检查输入是否包含潜在的注入攻击
    pub fn check_for_injection_attacks(input: &str) -> AppResult<()> {
        // SQL 注入检查
        let sql_patterns = [
            r"(?i)(union|select|insert|update|delete|drop|create|alter)",
            r"(?i)(script|javascript|vbscript)",
            r"(?i)(--|;|\/\*|\*\/)",
        ];

        for pattern in &sql_patterns {
            let re = Regex::new(pattern).unwrap();
            if re.is_match(input) {
                return Err(AppError::validation(
                    "Input contains potentially dangerous content",
                ));
            }
        }

        // XSS 检查
        if input.to_lowercase().contains("<script") {
            return Err(AppError::validation(
                "Input contains potentially dangerous script content",
            ));
        }

        Ok(())
    }

    /// 检查请求频率限制
    pub fn check_rate_limit(request_count: u32, limit: u32, window_seconds: u32) -> AppResult<()> {
        if request_count > limit {
            return Err(AppError::authentication(
                "Rate limit exceeded. Please try again later",
            ));
        }
        Ok(())
    }
}

// 正则表达式常量
lazy_static::lazy_static! {
    static ref ROOM_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9](?:[a-zA-Z0-9_-]{1,48}[a-zA-Z0-9])?$").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_name_validation() {
        assert!(RoomNameValidator::validate("test-room").is_ok());
        assert!(RoomNameValidator::validate("test_room").is_ok());
        assert!(RoomNameValidator::validate("test123").is_ok());

        assert!(RoomNameValidator::validate("").is_err());
        assert!(RoomNameValidator::validate("ab").is_err());
        assert!(RoomNameValidator::validate("-invalid").is_err());
        assert!(RoomNameValidator::validate("invalid-").is_err());
        assert!(RoomNameValidator::validate("invalid room").is_err());
    }

    #[test]
    fn test_password_validation() {
        assert!(PasswordValidator::validate("password123").is_ok());
        assert!(PasswordValidator::validate("MyPassword1").is_ok());

        assert!(PasswordValidator::validate("").is_err());
        assert!(PasswordValidator::validate("123456").is_err());
        assert!(PasswordValidator::validate("password").is_err());
    }

    #[test]
    fn test_filename_validation() {
        assert!(FilenameValidator::validate("document.pdf").is_ok());
        assert!(FilenameValidator::validate("image_file.jpg").is_ok());

        assert!(FilenameValidator::validate("").is_err());
        assert!(FilenameValidator::validate("../etc/passwd").is_err());
        assert!(FilenameValidator::validate("file\x00name").is_err());
        assert!(FilenameValidator::validate("CON").is_err());
    }

    #[test]
    fn test_filename_sanitization() {
        assert_eq!(
            FilenameValidator::sanitize("normal_file.txt"),
            "normal_file.txt"
        );
        assert_eq!(
            FilenameValidator::sanitize("file/with\\slashes"),
            "file_with_slashes"
        );
        assert_eq!(FilenameValidator::sanitize("  spaced.txt  "), "spaced.txt");
        assert_eq!(
            FilenameValidator::sanitize("file:with*invalid?chars"),
            "file_with_invalid_chars"
        );
    }

    #[test]
    fn test_token_validation() {
        assert!(TokenValidator::validate_token_format("header.payload.signature").is_ok());
        assert!(TokenValidator::validate_token_format("invalid").is_err());
        assert!(TokenValidator::validate_token_format("").is_err());

        assert!(TokenValidator::extract_from_auth_header("Bearer token123").is_ok());
        assert!(TokenValidator::extract_from_auth_header("Bearer").is_err());
        assert!(TokenValidator::extract_from_auth_header("Basic token123").is_err());
    }

    #[test]
    fn test_chunk_validation() {
        assert!(ChunkedUploadValidator::validate_chunk_params(1, 5, 1024, 5120).is_ok());
        assert!(ChunkedUploadValidator::validate_chunk_params(0, 5, 1024, 5120).is_err());
        assert!(ChunkedUploadValidator::validate_chunk_params(6, 5, 1024, 5120).is_err());
        assert!(ChunkedUploadValidator::validate_chunk_params(1, 0, 1024, 5120).is_err());
    }

    #[test]
    fn test_security_checker() {
        assert!(SecurityChecker::check_for_injection_attacks("normal text").is_ok());
        assert!(SecurityChecker::check_for_injection_attacks("'; DROP TABLE users; --").is_err());
        assert!(
            SecurityChecker::check_for_injection_attacks("<script>alert('xss')</script>").is_err()
        );
    }
}
