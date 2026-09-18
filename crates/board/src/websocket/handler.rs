//! WebSocket 连接握手处理器
//!
//! 负责校验 CONNECT 请求（令牌 + 能力），其余帧的收发由 `server` 模块处理

use crate::authz::{Authz, Resource};
use crate::handlers::verify_room_token;
use crate::models::room::role::Capability;
use crate::state::AppState;
use crate::websocket::types::{ConnectAck, ConnectRequest, RoomInfo, WsError};
use std::sync::Arc;

/// WebSocket 连接握手处理器
pub struct MessageHandler {
    app_state: AppState,
}

impl MessageHandler {
    /// 创建新的握手处理器
    pub fn new(app_state: AppState) -> Self {
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

        // WS 与 HTTP 共用 authz 判定：实时接收需要 msg.read。
        let authz = Authz::for_claims(&self.app_state, &verified.room, &verified.claims)
            .await
            .map_err(|error| WsError::InvalidToken(error.to_string()))?;
        authz
            .require(
                Capability::MsgRead,
                &Resource::Room {
                    room_id: verified.claims.room_id,
                },
            )
            .map_err(|_| WsError::PermissionDenied)?;

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
}
