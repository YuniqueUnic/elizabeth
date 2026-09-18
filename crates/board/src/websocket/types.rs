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

/// 房间更新原因
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoomUpdateReason {
    /// 房间访问地址已变更
    AddressChanged,
    /// 房间角色矩阵已变更（客户端应刷新能力判定）
    RolesChanged,
    /// 房间配置已变更
    SettingsChanged,
}

/// WebSocket 握手失败原因
///
/// 握手失败时服务端只发一帧 `{"error": "<Display 文案>"}` 再关闭连接，
/// 所以这里不是线上协议的一部分，只需覆盖 CONNECT 校验能产生的两种失败。
#[derive(Debug, Clone)]
pub enum WsError {
    /// 令牌缺失、无效、过期，或房间不存在
    InvalidToken(String),
    /// 令牌有效但缺少接收实时消息的能力
    PermissionDenied,
}

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            WsError::PermissionDenied => write!(f, "Permission denied"),
        }
    }
}

impl std::error::Error for WsError {}
