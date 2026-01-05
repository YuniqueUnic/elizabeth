//! WebSocket 服务器
//!
//! 提供 WebSocket 服务器功能和路由集成

use axum::extract::{ws::WebSocket, ws::WebSocketUpgrade, State};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::state::AppState;
use crate::websocket::{
    connection::ConnectionManager,
    handler::MessageHandler,
    types::{ConnectRequest, WsMessage, WsMessageType},
};

/// WebSocket 服务器
pub struct WsServer;

impl WsServer {
    /// 处理 WebSocket 连接升级
    pub async fn handle_ws(
        ws: WebSocketUpgrade,
        State(app_state): State<AppState>,
    ) -> impl axum::response::IntoResponse {
        // 升级连接到 WebSocket
        ws.on_upgrade(|socket| async move {
            Self::handle_socket(socket, app_state).await
        })
    }

    /// 处理 WebSocket socket
    async fn handle_socket(
        socket: WebSocket,
        app_state: AppState,
    ) {
        // 分离 socket 为 sink 和 stream
        let (sender, mut receiver) = socket.split();

        // 创建消息通道
        let (tx, rx) = mpsc::unbounded_channel::<WsMessage>();

        // 创建连接管理器和处理器
        let manager = ConnectionManager::new(app_state.clone());
        let handler = MessageHandler::new(app_state.clone(), manager);

        // TODO: 实现连接处理逻辑
        // 1. 接收连接请求
        // 2. 验证 token
        // 3. 订阅房间
        // 4. 开始消息循环

        // 消息处理循环
        while let Some(Ok(msg)) = receiver.next().await {
            // TODO: 处理接收到的消息
            let _: axum::extract::ws::Message = msg;
            log::info!("Received WebSocket message");
        }

        log::info!("WebSocket connection closed");
    }
}
