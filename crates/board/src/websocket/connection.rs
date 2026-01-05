//! WebSocket 连接管理器
//!
//! 管理 WebSocket 连接和房间订阅关系

use crate::state::AppState;
use crate::websocket::types::{WsMessage, WsMessageType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 连接管理器
pub struct ConnectionManager {
    app_state: AppState,
    /// 房间订阅关系: room_name -> connection_ids
    room_subscribers: RwLock<HashMap<String, Vec<String>>>,
    /// 活跃连接: connection_id -> (room_name, sender)
    connections: RwLock<HashMap<String, (String, tokio::sync::mpsc::UnboundedSender<WsMessage>)>>,
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new(app_state: AppState) -> Self {
        Self {
            app_state,
            room_subscribers: RwLock::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
        }
    }

    /// 订阅房间
    pub async fn subscribe_to_room(
        &self,
        connection_id: String,
        room_name: String,
        sender: tokio::sync::mpsc::UnboundedSender<WsMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut subscribers = self.room_subscribers.write().await;
        let mut connections = self.connections.write().await;

        // 添加到房间订阅者列表
        subscribers
            .entry(room_name.clone())
            .or_insert_with(Vec::new)
            .push(connection_id.clone());

        // 添加到活跃连接
        connections.insert(connection_id.clone(), (room_name.clone(), sender));

        log::info!(
            "Connection {} subscribed to room {}",
            connection_id,
            room_name
        );

        Ok(())
    }

    /// 向房间广播消息
    pub async fn broadcast_to_room(
        &self,
        room_name: &str,
        message: WsMessage,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let subscribers = self.room_subscribers.read().await;

        if let Some(connection_ids) = subscribers.get(room_name) {
            let connections = self.connections.read().await;
            let mut count = 0;

            for connection_id in connection_ids {
                if let Some((_, sender)) = connections.get(connection_id) {
                    if sender.send(message.clone()).is_ok() {
                        count += 1;
                    }
                }
            }

            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// 断开连接
    pub async fn disconnect(&self, connection_id: &str) {
        let mut subscribers = self.room_subscribers.write().await;
        let mut connections = self.connections.write().await;

        // 移除连接
        if let Some((room_name, _)) = connections.remove(connection_id) {
            // 从房间订阅者中移除
            if let Some(subscribers) = subscribers.get_mut(&room_name) {
                subscribers.retain(|id| id != connection_id);
            }

            log::info!(
                "Connection {} disconnected from room {}",
                connection_id,
                room_name
            );
        }
    }
}
