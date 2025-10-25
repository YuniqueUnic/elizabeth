use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

/// 应用错误类型
/// 统一的错误处理体系，涵盖所有可能的错误情况
#[derive(Debug, Error)]
pub enum AppError {
    /// 数据库相关错误
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// 认证相关错误
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// 授权相关错误
    #[error("Authorization failed: {message}")]
    Authorization { message: String },

    /// 输入验证错误
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// 房间不存在错误
    #[error("Room not found: {identifier}")]
    RoomNotFound { identifier: String },

    /// 权限不足错误
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    /// 文件上传错误
    #[error("File upload error: {message}")]
    FileUpload { message: String },

    /// 配置错误
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// 令牌相关错误
    #[error("Token error: {message}")]
    Token { message: String },

    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JWT 错误
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    /// 通用内部错误
    #[error("Internal server error: {message}")]
    Internal { message: String },

    /// 未找到错误
    #[error("Not found: {resource}")]
    NotFound { resource: String },

    /// 冲突错误
    #[error("Conflict: {message}")]
    Conflict { message: String },

    /// 请求超时错误
    #[error("Request timeout: {message}")]
    Timeout { message: String },

    /// 请求过大错误
    #[error("Payload too large: {message}")]
    PayloadTooLarge { message: String },

    /// 不支持的媒体类型错误
    #[error("Unsupported media type: {media_type}")]
    UnsupportedMediaType { media_type: String },
}

impl AppError {
    /// 获取错误对应的 HTTP 状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            AppError::Authorization { .. } => StatusCode::FORBIDDEN,
            AppError::Validation { .. } => StatusCode::BAD_REQUEST,
            AppError::RoomNotFound { .. } => StatusCode::NOT_FOUND,
            AppError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            AppError::FileUpload { .. } => StatusCode::BAD_REQUEST,
            AppError::Configuration { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Token { .. } => StatusCode::UNAUTHORIZED,
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Jwt(_) => StatusCode::UNAUTHORIZED,
            AppError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Conflict { .. } => StatusCode::CONFLICT,
            AppError::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
            AppError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            AppError::UnsupportedMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        }
    }

    /// 获取错误代码
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Authentication { .. } => "AUTHENTICATION_FAILED",
            AppError::Authorization { .. } => "AUTHORIZATION_FAILED",
            AppError::Validation { .. } => "VALIDATION_ERROR",
            AppError::RoomNotFound { .. } => "ROOM_NOT_FOUND",
            AppError::PermissionDenied { .. } => "PERMISSION_DENIED",
            AppError::FileUpload { .. } => "FILE_UPLOAD_ERROR",
            AppError::Configuration { .. } => "CONFIGURATION_ERROR",
            AppError::Token { .. } => "TOKEN_ERROR",
            AppError::Io(_) => "IO_ERROR",
            AppError::Jwt(_) => "JWT_ERROR",
            AppError::Internal { .. } => "INTERNAL_ERROR",
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::Conflict { .. } => "CONFLICT",
            AppError::Timeout { .. } => "TIMEOUT",
            AppError::PayloadTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            AppError::UnsupportedMediaType { .. } => "UNSUPPORTED_MEDIA_TYPE",
        }
    }

    /// 创建认证错误
    pub fn authentication(message: impl Into<String>) -> Self {
        AppError::Authentication {
            message: message.into(),
        }
    }

    /// 创建授权错误
    pub fn authorization(message: impl Into<String>) -> Self {
        AppError::Authorization {
            message: message.into(),
        }
    }

    /// 创建验证错误
    pub fn validation(message: impl Into<String>) -> Self {
        AppError::Validation {
            message: message.into(),
        }
    }

    /// 创建房间未找到错误
    pub fn room_not_found(identifier: impl Into<String>) -> Self {
        AppError::RoomNotFound {
            identifier: identifier.into(),
        }
    }

    /// 创建权限不足错误
    pub fn permission_denied(message: impl Into<String>) -> Self {
        AppError::PermissionDenied {
            message: message.into(),
        }
    }

    /// 创建文件上传错误
    pub fn file_upload(message: impl Into<String>) -> Self {
        AppError::FileUpload {
            message: message.into(),
        }
    }

    /// 创建配置错误
    pub fn configuration(message: impl Into<String>) -> Self {
        AppError::Configuration {
            message: message.into(),
        }
    }

    /// 创建令牌错误
    pub fn token(message: impl Into<String>) -> Self {
        AppError::Token {
            message: message.into(),
        }
    }

    /// 创建内部错误
    pub fn internal(message: impl Into<String>) -> Self {
        AppError::Internal {
            message: message.into(),
        }
    }

    /// 创建未找到错误
    pub fn not_found(resource: impl Into<String>) -> Self {
        AppError::NotFound {
            resource: resource.into(),
        }
    }

    /// 创建冲突错误
    pub fn conflict(message: impl Into<String>) -> Self {
        AppError::Conflict {
            message: message.into(),
        }
    }

    /// 创建超时错误
    pub fn timeout(message: impl Into<String>) -> Self {
        AppError::Timeout {
            message: message.into(),
        }
    }

    /// 创建请求过大错误
    pub fn payload_too_large(message: impl Into<String>) -> Self {
        AppError::PayloadTooLarge {
            message: message.into(),
        }
    }

    /// 创建不支持的媒体类型错误
    pub fn unsupported_media_type(media_type: impl Into<String>) -> Self {
        AppError::UnsupportedMediaType {
            media_type: media_type.into(),
        }
    }
}

/// 实现 IntoResponse trait，用于 Axum 响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "status": status.as_u16()
            }
        });

        (status, Json(error_response)).into_response()
    }
}

/// 为 axum_responses::http::HttpResponse 实现转换
impl From<AppError> for axum_responses::http::HttpResponse {
    fn from(err: AppError) -> Self {
        let status = err.status_code();
        let error_response = json!({
            "error": {
                "code": err.error_code(),
                "message": err.to_string(),
                "status": status.as_u16()
            }
        });

        match status.as_u16() {
            400 => axum_responses::http::HttpResponse::BadRequest().message(err.to_string()),
            401 => axum_responses::http::HttpResponse::Unauthorized().message(err.to_string()),
            403 => axum_responses::http::HttpResponse::Forbidden().message(err.to_string()),
            404 => axum_responses::http::HttpResponse::NotFound().message(err.to_string()),
            409 => axum_responses::http::HttpResponse::Conflict().message(err.to_string()),
            500 => {
                axum_responses::http::HttpResponse::InternalServerError().message(err.to_string())
            }
            _ => axum_responses::http::HttpResponse::InternalServerError().message(err.to_string()),
        }
    }
}

/// 从 anyhow::Error 转换为 AppError
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        // 尝试匹配特定错误类型
        if let Some(db_err) = err.downcast_ref::<sqlx::Error>() {
            return AppError::internal(format!("Database error: {}", db_err));
        }

        if let Some(jwt_err) = err.downcast_ref::<jsonwebtoken::errors::Error>() {
            return AppError::token(format!("JWT error: {}", jwt_err));
        }

        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return AppError::internal(format!("IO error: {}", io_err));
        }

        // 默认作为内部错误处理
        AppError::Internal {
            message: err.to_string(),
        }
    }
}

/// 应用结果类型
/// 对于 API 处理器，使用 Result<T, AppError>
pub type AppResult<T> = Result<T, AppError>;

/// anyhow 结果类型（用于非 API 代码）
/// 对于内部逻辑，使用 Result<T, anyhow::Error>
pub type AnyhowResult<T> = Result<T, anyhow::Error>;

/// 错误处理工具函数
pub mod utils {
    use super::*;

    /// 将 anyhow::Result 转换为 AppResult
    pub fn map_anyhow_to_app<T>(result: AnyhowResult<T>) -> AppResult<T> {
        result.map_err(AppError::from)
    }

    /// 将 AppError 转换为 anyhow::Error
    pub fn map_app_to_anyhow<T>(result: AppResult<T>) -> AnyhowResult<T> {
        result.map_err(|err| anyhow::Error::msg(err.to_string()))
    }

    /// 在错误上下文中添加信息
    pub fn with_context<T, E>(result: Result<T, E>, context: &str) -> AppResult<T>
    where
        E: Into<AppError>,
    {
        result.map_err(|err| {
            let app_err = err.into();
            match app_err {
                AppError::Validation { message } => {
                    AppError::validation(format!("{}: {}", context, message))
                }
                AppError::Authentication { message } => {
                    AppError::authentication(format!("{}: {}", context, message))
                }
                AppError::Authorization { message } => {
                    AppError::authorization(format!("{}: {}", context, message))
                }
                _ => AppError::internal(format!("{}: {}", context, app_err)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = AppError::authentication("Invalid token");
        assert_eq!(err.error_code(), "AUTHENTICATION_FAILED");
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_error_response_format() {
        let err = AppError::validation("Invalid input");
        let response = err.into_response();

        // 这里可以进一步测试响应格式
        assert!(response.status().is_client_error());
    }

    #[test]
    fn test_anyhow_conversion() {
        let anyhow_err = anyhow::anyhow!("Something went wrong");
        let app_err = AppError::from(anyhow_err);

        match app_err {
            AppError::Internal { message } => {
                assert!(message.contains("Something went wrong"));
            }
            _ => panic!("Expected Internal error"),
        }
    }
}
