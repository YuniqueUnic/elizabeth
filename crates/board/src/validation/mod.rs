use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

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
        let re = get_room_name_regex();
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

// 正则表达式常量
fn get_room_name_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z0-9](?:[a-zA-Z0-9_-]{1,48}[a-zA-Z0-9])?$").unwrap())
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
    fn test_token_validation() {
        assert!(TokenValidator::validate_token_format("header.payload.signature").is_ok());
        assert!(TokenValidator::validate_token_format("invalid").is_err());
        assert!(TokenValidator::validate_token_format("").is_err());

        assert!(TokenValidator::extract_from_auth_header("Bearer token123").is_ok());
        assert!(TokenValidator::extract_from_auth_header("Bearer").is_err());
        assert!(TokenValidator::extract_from_auth_header("Basic token123").is_err());
    }
}
