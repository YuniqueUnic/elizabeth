//! WebSocket 服务器
//!
//! 提供 WebSocket 服务器功能和路由集成

use axum::extract::{State, ws::WebSocket, ws::WebSocketUpgrade};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

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
        ws.on_upgrade(|socket| async move { Self::handle_socket(socket, app_state).await })
    }

    /// 处理 WebSocket socket
    async fn handle_socket(socket: WebSocket, app_state: AppState) {
        // 分离 socket 为 sink 和 stream
        let (mut sender, mut receiver) = socket.split();

        // 创建消息通道用于接收广播
        let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

        // 生成唯一连接 ID
        let connection_id = Uuid::new_v4().to_string();

        // 使用共享的连接管理器
        let manager = app_state.connection_manager.clone();
        let handler = MessageHandler::new(app_state.clone(), manager.clone());

        // 接收第一条 CONNECT 消息
        let room_name =
            match Self::handle_connect_handshake(&mut receiver, &handler, &mut sender).await {
                Ok(room_name) => room_name,
                Err(e) => {
                    log::error!("Connect handshake failed: {}", e);
                    let error_msg = format!("Connection failed: {}", e);
                    let _ = sender
                        .send(axum::extract::ws::Message::Text(
                            serde_json::to_string(&WsMessage::error(&error_msg))
                                .unwrap_or_default()
                                .into(),
                        ))
                        .await;
                    return;
                }
            };

        // 订阅房间
        let room_name_for_log = room_name.clone();
        let error_to_send: Option<String> = match manager
            .subscribe_to_room(connection_id.clone(), room_name.clone(), tx)
            .await
        {
            Ok(_) => None,
            Err(e) => {
                // 立即转换为 String，避免 Box<dyn StdError> 跨越 await
                let msg = format!("Subscription failed: {}", e);
                log::error!("Failed to subscribe to room {}: {}", room_name_for_log, msg);
                Some(msg)
            }
        };
        // subscribe_result 已被 match 消费，不再存在

        if let Some(error_msg) = error_to_send {
            let error_response =
                serde_json::to_string(&WsMessage::error(&error_msg)).unwrap_or_default();
            let _ = sender
                .send(axum::extract::ws::Message::Text(error_response.into()))
                .await;
            return;
        }

        log::info!(
            "Connection {} established for room {}",
            connection_id,
            room_name
        );

        // 创建接收客户端消息的任务
        let manager_recv = manager.clone();
        let connection_id_recv = connection_id.clone();
        let room_name_recv = room_name.clone();
        let recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                if let Err(e) = Self::handle_client_message(
                    msg,
                    &handler,
                    &manager_recv,
                    &connection_id_recv,
                    &room_name_recv,
                )
                .await
                {
                    log::error!("Error handling client message: {}", e);
                    break;
                }
            }
            log::debug!("Receive task ended for connection {}", connection_id_recv);
        });

        // 创建发送广播消息的任务
        let connection_id_send = connection_id.clone();
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };

                if sender
                    .send(axum::extract::ws::Message::Text(json.into()))
                    .await
                    .is_err()
                {
                    log::error!(
                        "Failed to send message to connection {}",
                        connection_id_send
                    );
                    break;
                }
            }
            log::debug!("Send task ended for connection {}", connection_id_send);
        });

        // 等待任一任务完成
        tokio::select! {
            _ = recv_task => {
                log::info!("Receive task completed for connection {}", connection_id);
            }
            _ = send_task => {
                log::info!("Send task completed for connection {}", connection_id);
            }
        }

        // 清理连接
        manager.disconnect(&connection_id).await;
        log::info!("WebSocket connection {} closed", connection_id);
    }

    /// 处理连接握手
    async fn handle_connect_handshake(
        receiver: &mut futures::stream::SplitStream<WebSocket>,
        handler: &MessageHandler,
        sender: &mut futures::stream::SplitSink<WebSocket, axum::extract::ws::Message>,
    ) -> Result<String, String> {
        // 接收第一条消息
        let first_msg = match receiver.next().await {
            Some(Ok(msg)) => msg,
            Some(Err(e)) => {
                return Err(format!("Failed to receive first message: {}", e));
            }
            None => {
                return Err("Connection closed before first message".to_string());
            }
        };

        // 解析文本消息
        let text = match first_msg {
            axum::extract::ws::Message::Text(t) => t,
            axum::extract::ws::Message::Close(_) => {
                return Err("Connection closed by client".to_string());
            }
            _ => {
                return Err("First message must be text".to_string());
            }
        };

        // 解析 WsMessage
        let ws_msg: WsMessage =
            serde_json::from_str(&text).map_err(|e| format!("Failed to parse WsMessage: {}", e))?;

        // 处理 CONNECT 消息
        if ws_msg.message_type != WsMessageType::Connect {
            return Err("First message must be CONNECT".to_string());
        }

        // 解析 ConnectRequest
        let connect_req: ConnectRequest = serde_json::from_value(
            ws_msg
                .payload
                .ok_or_else(|| "Missing payload in CONNECT message".to_string())?,
        )
        .map_err(|e| format!("Failed to parse ConnectRequest: {}", e))?;

        // 验证连接请求
        let ack = handler
            .handle_connect(connect_req.clone())
            .await
            .map_err(|e| format!("Connect verification failed: {}", e))?;

        // 发送确认消息
        let ack_msg = WsMessage::new(
            WsMessageType::ConnectAck,
            Some(
                serde_json::to_value(ack)
                    .map_err(|e| format!("Failed to serialize ConnectAck: {}", e))?,
            ),
        );
        let ack_json = serde_json::to_string(&ack_msg)
            .map_err(|e| format!("Failed to serialize WsMessage: {}", e))?;

        sender
            .send(axum::extract::ws::Message::Text(ack_json.into()))
            .await
            .map_err(|e| format!("Failed to send CONNECT_ACK: {}", e))?;

        Ok(connect_req.room_name)
    }

    /// 处理客户端消息
    async fn handle_client_message(
        msg: axum::extract::ws::Message,
        _handler: &MessageHandler,
        _manager: &ConnectionManager,
        _connection_id: &str,
        _room_name: &str,
    ) -> Result<(), String> {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                // 解析 WsMessage
                let ws_msg: WsMessage = serde_json::from_str(&text)
                    .map_err(|e| format!("Failed to parse message: {}", e))?;

                // 处理不同类型的消息
                match ws_msg.message_type {
                    WsMessageType::Ping => {
                        // PING 消息自动回复 PONG
                        log::debug!("Received PING, sending PONG");
                    }
                    WsMessageType::Pong => {
                        log::debug!("Received PONG");
                    }
                    _ => {
                        log::debug!("Received message type: {:?}", ws_msg.message_type);
                    }
                }

                Ok(())
            }
            axum::extract::ws::Message::Close(_) => {
                log::info!("Client initiated close");
                Err("Connection closed by client".to_string())
            }
            _ => {
                log::debug!("Received non-text message");
                Ok(())
            }
        }
    }
}
