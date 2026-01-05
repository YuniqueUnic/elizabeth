//! WebSocket 模块
//!
//! 提供实时通信功能，支持房间级别的消息广播和用户连接管理

pub mod broadcaster;
pub mod connection;
pub mod handler;
pub mod server;
pub mod types;

// 重新导出主要类型
pub use types::{WsMessage, WsMessageType, ConnectRequest, ConnectAck, WsError};
