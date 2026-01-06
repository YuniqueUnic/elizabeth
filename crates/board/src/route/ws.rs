//! WebSocket 路由模块
//!
//! 提供 WebSocket 连接的路由处理器

use axum::{Router, routing::get};
use std::sync::Arc;

use crate::state::AppState;
use crate::websocket::server::WsServer;

/// 创建 WebSocket 路由
///
/// 返回一个包含 `/ws` 端点的路由器
pub fn api_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(WsServer::handle_ws))
        .with_state((*app_state).clone())
}
