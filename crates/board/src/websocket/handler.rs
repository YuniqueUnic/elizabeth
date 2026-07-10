//! WebSocket 消息处理器
//!
//! 处理 WebSocket 消息和认证

use crate::handlers::verify_room_token;
use crate::models::room::permission::RoomPermission;
use crate::state::AppState;
use crate::websocket::{
    connection::ConnectionManager,
    types::{ConnectAck, ConnectRequest, RoomInfo, WsError, WsMessage, WsMessageType},
};
use std::sync::Arc;

/// WebSocket 消息处理器
pub struct MessageHandler {
    app_state: AppState,
}

impl MessageHandler {
    /// 创建新的消息处理器
    pub fn new(app_state: AppState, _manager: Arc<ConnectionManager>) -> Self {
        Self { app_state }
    }

    /// 处理连接请求
    pub async fn handle_connect(&self, request: ConnectRequest) -> Result<ConnectAck, WsError> {
        log::info!("Connect request from room_name: {}", request.room_name);

        let verified = verify_room_token(
            Arc::new(self.app_state.clone()),
            &request.room_name,
            &request.token,
        )
        .await
        .map_err(|error| WsError::InvalidToken(error.to_string()))?;
        if !verified.room.permission.contains(RoomPermission::VIEW_ONLY)
            || !verified
                .claims
                .as_permission()
                .contains(RoomPermission::VIEW_ONLY)
        {
            return Err(WsError::PermissionDenied);
        }

        log::info!(
            "Token verified successfully for room_id: {}, room_name: {}",
            verified.claims.room_id,
            verified.claims.room_name
        );

        let room = verified.room;
        let room_info = Some(RoomInfo {
            id: room.id.unwrap_or_default(),
            name: room.name,
            slug: room.slug,
            max_size: room.max_size,
            current_size: room.current_size,
            max_times_entered: room.max_times_entered,
            current_times_entered: room.current_times_entered,
        });

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
