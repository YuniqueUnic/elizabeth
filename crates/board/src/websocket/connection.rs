//! WebSocket 连接管理器
//!
//! 管理 WebSocket 连接和房间订阅关系，支持性能优化和资源限制

use crate::websocket::types::{WsMessage, WsMessageType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 连接管理器配置
#[derive(Debug, Clone)]
pub struct ConnectionManagerConfig {
    /// 单个房间最大连接数
    pub max_connections_per_room: usize,
    /// 全局最大连接数
    pub max_global_connections: usize,
    /// 是否启用连接统计
    pub enable_metrics: bool,
}

impl Default for ConnectionManagerConfig {
    fn default() -> Self {
        Self {
            max_connections_per_room: 100,
            max_global_connections: 1000,
            enable_metrics: true,
        }
    }
}

/// 连接统计信息
#[derive(Debug, Default, Clone)]
pub struct ConnectionMetrics {
    /// 当前活跃连接数
    pub active_connections: usize,
    /// 当前房间数
    pub active_rooms: usize,
    /// 累计连接数
    pub total_connections: usize,
    /// 累计断开连接数
    pub total_disconnections: usize,
}

/// 连接管理器
pub struct ConnectionManager {
    /// 配置
    config: ConnectionManagerConfig,
    /// 房间订阅关系：room_name -> connection_ids
    room_subscribers: RwLock<HashMap<String, Vec<String>>>,
    /// 活跃连接：connection_id -> (room_name, sender)
    connections: RwLock<HashMap<String, (String, tokio::sync::mpsc::UnboundedSender<WsMessage>)>>,
    /// 连接统计
    metrics: RwLock<ConnectionMetrics>,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self::with_config(ConnectionManagerConfig::default())
    }

    /// 使用配置创建连接管理器
    pub fn with_config(config: ConnectionManagerConfig) -> Self {
        Self {
            config,
            room_subscribers: RwLock::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
            metrics: RwLock::new(ConnectionMetrics::default()),
        }
    }

    /// 订阅房间
    pub async fn subscribe_to_room(
        &self,
        connection_id: String,
        room_name: String,
        sender: tokio::sync::mpsc::UnboundedSender<WsMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 检查全局连接数限制
        {
            let connections = self.connections.read().await;
            if connections.len() >= self.config.max_global_connections {
                return Err("全局连接数已达上限".into());
            }
        }

        let mut subscribers = self.room_subscribers.write().await;
        let mut connections = self.connections.write().await;
        let mut metrics = self.metrics.write().await;

        // 检查房间连接数限制
        if let Some(existing_connections) = subscribers.get(&room_name) {
            if existing_connections.len() >= self.config.max_connections_per_room {
                return Err(format!("房间 {} 连接数已达上限", room_name).into());
            }
        }

        // 添加到房间订阅者列表
        subscribers
            .entry(room_name.clone())
            .or_insert_with(Vec::new)
            .push(connection_id.clone());

        // 添加到活跃连接
        connections.insert(connection_id.clone(), (room_name.clone(), sender));

        // 更新统计
        metrics.active_connections = connections.len();
        metrics.active_rooms = subscribers.len();
        metrics.total_connections += 1;

        log::info!(
            "Connection {} subscribed to room {} (total: {}, rooms: {})",
            connection_id,
            room_name,
            metrics.active_connections,
            metrics.active_rooms
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
            let mut failed_ids = Vec::new();

            for connection_id in connection_ids {
                if let Some((_, sender)) = connections.get(connection_id) {
                    if sender.send(message.clone()).is_ok() {
                        count += 1;
                    } else {
                        // 记录发送失败的连接 ID
                        failed_ids.push(connection_id.clone());
                    }
                }
            }

            // 清理失败的连接
            if !failed_ids.is_empty() {
                drop(connections);
                drop(subscribers);
                for failed_id in failed_ids {
                    self.disconnect(&failed_id).await;
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
        let mut metrics = self.metrics.write().await;

        // 移除连接
        if let Some((room_name, _)) = connections.remove(connection_id) {
            // 从房间订阅者中移除
            if let Some(subscribers) = subscribers.get_mut(&room_name) {
                subscribers.retain(|id| id != connection_id);
            }

            // 如果房间没有订阅者了，移除房间
            if subscribers
                .get(&room_name)
                .map(|v| v.is_empty())
                .unwrap_or(false)
            {
                subscribers.remove(&room_name);
            }

            // 更新统计
            metrics.active_connections = connections.len();
            metrics.active_rooms = subscribers.len();
            metrics.total_disconnections += 1;

            log::info!(
                "Connection {} disconnected from room {} (total: {}, rooms: {})",
                connection_id,
                room_name,
                metrics.active_connections,
                metrics.active_rooms
            );
        }
    }

    /// 获取连接统计信息
    pub async fn get_metrics(&self) -> ConnectionMetrics {
        self.metrics.read().await.clone()
    }

    /// 获取房间连接数
    pub async fn get_room_connection_count(&self, room_name: &str) -> usize {
        let subscribers = self.room_subscribers.read().await;
        subscribers.get(room_name).map(|v| v.len()).unwrap_or(0)
    }

    /// 获取所有房间列表
    pub async fn get_active_rooms(&self) -> Vec<String> {
        let subscribers = self.room_subscribers.read().await;
        subscribers.keys().cloned().collect()
    }

    /// 清理所有连接（用于测试或重启）
    pub async fn cleanup_all(&self) {
        let mut subscribers = self.room_subscribers.write().await;
        let mut connections = self.connections.write().await;
        let mut metrics = self.metrics.write().await;

        let count = connections.len();
        connections.clear();
        subscribers.clear();

        metrics.active_connections = 0;
        metrics.active_rooms = 0;

        log::info!("Cleaned up {} connections", count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_connection_limits() {
        let config = ConnectionManagerConfig {
            max_connections_per_room: 2,
            max_global_connections: 5,
            enable_metrics: true,
        };
        let manager = ConnectionManager::with_config(config);

        let room_name = "test-room".to_string();

        // 添加 2 个连接应该成功
        for i in 0..2 {
            let (tx, _rx) = mpsc::unbounded_channel::<WsMessage>();
            let result = manager
                .subscribe_to_room(format!("conn-{}", i), room_name.clone(), tx)
                .await;
            assert!(result.is_ok(), "连接 {} 应该成功", i);
        }

        // 第 3 个连接应该失败
        let (tx, _rx) = mpsc::unbounded_channel::<WsMessage>();
        let result = manager
            .subscribe_to_room("conn-2".to_string(), room_name.clone(), tx)
            .await;
        assert!(result.is_err(), "超过房间连接限制应该失败");
    }

    #[tokio::test]
    async fn test_metrics() {
        let manager = ConnectionManager::new();
        let room_name = "metrics-test-room".to_string();

        let (tx1, _rx1) = mpsc::unbounded_channel::<WsMessage>();
        let (tx2, _rx2) = mpsc::unbounded_channel::<WsMessage>();

        manager
            .subscribe_to_room("conn-1".to_string(), room_name.clone(), tx1)
            .await
            .unwrap();
        manager
            .subscribe_to_room("conn-2".to_string(), room_name.clone(), tx2)
            .await
            .unwrap();

        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.active_connections, 2);
        assert_eq!(metrics.active_rooms, 1);
        assert_eq!(metrics.total_connections, 2);

        manager.disconnect("conn-1").await;

        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.active_connections, 1);
        assert_eq!(metrics.total_disconnections, 1);
    }
}
