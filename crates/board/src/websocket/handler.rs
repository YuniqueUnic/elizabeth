//! WebSocket 消息处理器
//!
//! 处理 WebSocket 消息和认证

use crate::state::AppState;
use crate::websocket::{
    connection::ConnectionManager,
    types::{ConnectAck, ConnectRequest, RoomInfo, WsError, WsMessage, WsMessageType},
};
use std::sync::Arc;

/// WebSocket 消息处理器
pub struct MessageHandler {
    app_state: AppState,
    manager: Arc<ConnectionManager>,
}

impl MessageHandler {
    /// 创建新的消息处理器
    pub fn new(app_state: AppState, manager: Arc<ConnectionManager>) -> Self {
        Self { app_state, manager }
    }

    /// 处理连接请求
    pub async fn handle_connect(&self, request: ConnectRequest) -> Result<ConnectAck, WsError> {
        log::info!("Connect request from room_name: {}", request.room_name);

        // 验证 token（JWT 格式）
        let claims = self
            .app_state
            .token_service()
            .decode(&request.token)
            .map_err(|e| WsError::InvalidToken(format!("Token verification failed: {}", e)))?;

        // 检查 token 是否过期
        if claims.is_expired() {
            return Err(WsError::InvalidToken("Token is expired".to_string()));
        }

        // 验证房间名称是否匹配
        if claims.room_name != request.room_name {
            return Err(WsError::InvalidToken("Room name mismatch".to_string()));
        }

        log::info!(
            "Token verified successfully for room_id: {}, room_name: {}",
            claims.room_id,
            claims.room_name
        );

        // TODO: 获取房间信息并验证房间名称
        let room_info = None;

        Ok(ConnectAck {
            success: true,
            message: "Connected successfully".to_string(),
            room_info,
        })
    }

    /// 处理 PING 消息
    pub fn handle_ping(&self) -> WsMessage {
        WsMessage::new(WsMessageType::Pong, None)
    }

    /// 处理消息
    pub async fn handle_message(&self, message: WsMessage) -> Result<WsMessage, WsError> {
        match message.message_type {
            WsMessageType::Connect => {
                // 解析连接请求
                if let Some(payload) = message.payload {
                    if let Ok(request) = serde_json::from_value::<ConnectRequest>(payload) {
                        let ack = self.handle_connect(request).await?;
                        Ok(WsMessage::new(
                            WsMessageType::ConnectAck,
                            Some(serde_json::to_value(ack)?),
                        ))
                    } else {
                        Ok(WsMessage::error("Invalid connect request"))
                    }
                } else {
                    Ok(WsMessage::error("Missing connect payload"))
                }
            }
            WsMessageType::Ping => Ok(self.handle_ping()),
            WsMessageType::Pong => Ok(WsMessage::new(WsMessageType::Pong, None)),
            _ => Ok(WsMessage::error("Unhandled message type")),
        }
    }

    /// 发送消息到客户端
    pub fn send_message(&self, message: WsMessage) -> WsMessage {
        message
    }

    /// 发送错误消息
    pub fn send_error(&self, error: WsError) -> WsMessage {
        WsMessage::error(&error.to_string())
    }
}
