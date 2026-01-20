//! WebSocket 类型定义
//!
//! 定义 WebSocket 通信使用的所有消息和错误类型

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// WebSocket 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsMessageType {
    /// 连接请求
    Connect,
    /// 连接确认
    ConnectAck,
    /// 心跳
    Ping,
    /// 心跳响应
    Pong,
    /// 错误
    Error,
    /// 内容创建事件
    ContentCreated,
    /// 内容更新事件
    ContentUpdated,
    /// 内容删除事件
    ContentDeleted,
    /// 用户加入房间事件
    UserJoined,
    /// 用户离开房间事件
    UserLeft,
    /// 房间更新事件
    RoomUpdate,
}

/// WebSocket 消息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct WsMessage {
    pub message_type: WsMessageType,
    pub payload: Option<serde_json::Value>,
    pub timestamp: i64,
}

impl WsMessage {
    /// 创建新的 WebSocket 消息
    pub fn new(message_type: WsMessageType, payload: Option<serde_json::Value>) -> Self {
        Self {
            message_type,
            payload,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// 创建错误消息
    pub fn error(error: &str) -> Self {
        Self {
            message_type: WsMessageType::Error,
            payload: Some(serde_json::json!({ "error": error })),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// 连接请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConnectRequest {
    pub token: String,
    pub room_name: String,
}

/// 连接确认
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConnectAck {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_info: Option<RoomInfo>,
}

/// 房间信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RoomInfo {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub max_size: i64,
    pub current_size: i64,
    pub max_times_entered: i64,
    pub current_times_entered: i64,
}

/// WebSocket 错误类型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WsError {
    /// 无效的 token
    InvalidToken(String),
    /// 房间不存在
    RoomNotFound,
    /// 权限不足
    PermissionDenied,
    /// 无效的消息格式
    InvalidMessage(String),
    /// 内部错误
    InternalError(String),
}

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            WsError::RoomNotFound => write!(f, "Room not found"),
            WsError::PermissionDenied => write!(f, "Permission denied"),
            WsError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
            WsError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for WsError {}

impl From<serde_json::Error> for WsError {
    fn from(err: serde_json::Error) -> Self {
        WsError::InvalidMessage(err.to_string())
    }
}
