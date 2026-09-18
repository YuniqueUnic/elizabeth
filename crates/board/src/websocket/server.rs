//! WebSocket 服务器
//!
//! 提供 WebSocket 服务器功能和路由集成

use std::time::Duration;

use axum::extract::{State, ws::WebSocket, ws::WebSocketUpgrade};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::state::AppState;
use crate::websocket::{
    handler::MessageHandler,
    types::{ConnectRequest, WsMessage, WsMessageType},
};

/// 心跳间隔。
///
/// 前端在 30s + 5s 余量内收不到 PING 就判定连接已死，主动关闭并重连
/// （见 `web/lib/hooks/use-websocket.ts` 的 `WS_CONFIG.HEARTBEAT_INTERVAL`），
/// 所以这里必须留出足够余量。
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(25);

/// WebSocket 服务器
pub struct WsServer;

impl WsServer {
    /// 周期性把 PING 推进连接的消息通道。
    ///
    /// 连接结束后通道接收端被丢弃，`send` 失败即自然退出，无需额外的取消信号。
    pub async fn send_heartbeats(tx: mpsc::UnboundedSender<WsMessage>, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // interval 的首次 tick 立即就绪，跳过它，避免紧接着 CONNECT_ACK 再补一帧
        ticker.tick().await;

        loop {
            ticker.tick().await;
            if tx.send(WsMessage::new(WsMessageType::Ping, None)).is_err() {
                return;
            }
        }
    }

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
        // 心跳需要独立的发送端：subscribe_to_room 会消费 tx
        let heartbeat_tx = tx.clone();

        // 生成唯一连接 ID
        let connection_id = Uuid::new_v4().to_string();

        // 使用共享的连接管理器
        let manager = app_state.connection_manager.clone();
        let handler = MessageHandler::new(app_state.clone());
        let room_lifecycle = app_state.services.room_lifecycle.clone();

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

        if let Err(e) = room_lifecycle.on_room_became_active(&room_name).await {
            log::warn!("Failed to clear room gc markers for {}: {}", room_name, e);
        }

        // 创建接收客户端消息的任务。
        // 握手之后客户端只会发 PONG（以及关闭帧），所以这里只负责消费并记录；
        // 收到关闭帧或无法解析的帧即结束接收，由下方 select! 触发整条连接的清理。
        let connection_id_recv = connection_id.clone();
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(frame)) = receiver.next().await {
                match frame {
                    axum::extract::ws::Message::Close(_) => {
                        log::info!("Client closed connection {}", connection_id_recv);
                        break;
                    }
                    axum::extract::ws::Message::Text(text) => {
                        match serde_json::from_str::<WsMessage>(&text) {
                            Ok(message) => log::debug!(
                                "Received {:?} from connection {}",
                                message.message_type,
                                connection_id_recv
                            ),
                            Err(e) => {
                                log::warn!(
                                    "Malformed frame on connection {}: {}",
                                    connection_id_recv,
                                    e
                                );
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            log::debug!("Receive task ended for connection {}", connection_id_recv);
        });

        // 创建发送广播消息的任务
        let connection_id_send = connection_id.clone();
        let mut send_task = tokio::spawn(async move {
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

        // 创建心跳任务
        let heartbeat_task = tokio::spawn(Self::send_heartbeats(heartbeat_tx, HEARTBEAT_INTERVAL));

        // 等待任一任务完成
        tokio::select! {
            _ = &mut recv_task => {
                log::info!("Receive task completed for connection {}", connection_id);
                send_task.abort();
                heartbeat_task.abort();
                let _ = send_task.await;
                let _ = heartbeat_task.await;
            }
            _ = &mut send_task => {
                log::info!("Send task completed for connection {}", connection_id);
                recv_task.abort();
                heartbeat_task.abort();
                let _ = recv_task.await;
                let _ = heartbeat_task.await;
            }
        }

        // 清理连接
        manager.disconnect(&connection_id).await;
        if manager.get_room_connection_count(&room_name).await == 0
            && let Err(e) = room_lifecycle.on_room_became_empty(&room_name).await
        {
            log::warn!("Failed to mark room {} for gc: {}", room_name, e);
        }
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
}
