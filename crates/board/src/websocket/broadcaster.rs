//! 房间事件广播器
//!
//! 向房间内所有订阅者广播事件

use crate::models::content::RoomContent;
use crate::websocket::connection::ConnectionManager;
use crate::websocket::types::{RoomInfo, RoomUpdateReason, WsMessage, WsMessageType};
use serde_json::json;
use std::sync::Arc;

/// 房间事件广播器
pub struct Broadcaster {
    manager: Arc<ConnectionManager>,
}

impl Broadcaster {
    /// 创建新的广播器
    pub fn new(manager: Arc<ConnectionManager>) -> Self {
        Self { manager }
    }

    /// 广播内容创建事件
    pub async fn broadcast_content_created(
        &self,
        room_name: &str,
        content: &RoomContent,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let payload = json!({
            "content_id": content.id,
            "room_name": room_name,
            "content_type": content.content_type,
            "text": content.text,
            "file_name": content.file_name,
            "sequence_number": content.sequence_number,
            "created_at": content.created_at,
            "updated_at": content.updated_at,
        });

        let message = WsMessage::new(WsMessageType::ContentCreated, Some(payload));

        self.manager.broadcast_to_room(room_name, message).await
    }

    /// 广播内容更新事件。
    ///
    /// 事件广播到全房间，隐藏内容不得经由 payload 泄漏正文：
    /// hidden = true 时省略 text/file_name，客户端收到后只需失效查询缓存，
    /// 可见性过滤由各自的列表接口按能力完成。
    pub async fn broadcast_content_updated(
        &self,
        room_name: &str,
        content: &RoomContent,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let payload = json!({
            "content_id": content.id,
            "room_name": room_name,
            "content_type": content.content_type,
            "text": if content.hidden { None } else { content.text.clone() },
            "file_name": if content.hidden { None } else { content.file_name.clone() },
            "hidden": content.hidden,
            "sequence_number": content.sequence_number,
            "created_at": content.created_at,
            "updated_at": content.updated_at,
        });

        let message = WsMessage::new(WsMessageType::ContentUpdated, Some(payload));

        self.manager.broadcast_to_room(room_name, message).await
    }

    /// 广播内容删除事件
    pub async fn broadcast_content_deleted(
        &self,
        room_name: &str,
        content: &RoomContent,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let payload = json!({
            "content_id": content.id,
            "room_name": room_name,
            "content_type": content.content_type,
            "text": content.text,
            "file_name": content.file_name,
            "sequence_number": content.sequence_number,
            "created_at": content.created_at,
            "updated_at": content.updated_at,
        });

        let message = WsMessage::new(WsMessageType::ContentDeleted, Some(payload));

        self.manager.broadcast_to_room(room_name, message).await
    }

    /// 广播用户加入事件
    pub async fn broadcast_user_joined(
        &self,
        room_name: &str,
        user_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let payload = json!({
            "user_id": user_id,
            "room_name": room_name,
        });

        let message = WsMessage::new(WsMessageType::UserJoined, Some(payload));

        self.manager.broadcast_to_room(room_name, message).await
    }

    /// 广播用户离开事件
    pub async fn broadcast_user_left(
        &self,
        room_name: &str,
        user_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let payload = json!({
            "user_id": user_id,
            "room_name": room_name,
        });

        let message = WsMessage::new(WsMessageType::UserLeft, Some(payload));

        self.manager.broadcast_to_room(room_name, message).await
    }

    /// 广播房间更新事件
    pub async fn broadcast_room_update(
        &self,
        room_name: &str,
        room_info: &RoomInfo,
        reason: RoomUpdateReason,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let payload = json!({
            "room_name": room_name,
            "room_info": room_info,
            "reason": reason,
        });

        let message = WsMessage::new(WsMessageType::RoomUpdate, Some(payload));

        self.manager.broadcast_to_room(room_name, message).await
    }
}
