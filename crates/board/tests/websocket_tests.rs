//! WebSocket 模块单元测试
//!
//! 测试 ConnectionManager、Broadcaster 和 MessageHandler

use board::models::room::content::{ContentType, RoomContent};
use board::websocket::broadcaster::Broadcaster;
use board::websocket::connection::ConnectionManager;
use board::websocket::types::{RoomInfo, WsError, WsMessage, WsMessageType};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::mpsc;

// ============================================================================
// ConnectionManager 测试
// ============================================================================

#[tokio::test]
async fn test_connection_manager_new() {
    let _manager = ConnectionManager::new();
    // 测试创建成功
    assert!(true, "ConnectionManager::new() should succeed");
}

#[tokio::test]
async fn test_subscribe_to_room() {
    let manager = ConnectionManager::new();
    let room_name = "test-room".to_string();
    let connection_id = "conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel::<WsMessage>();

    let result = manager
        .subscribe_to_room(connection_id, room_name, tx)
        .await;

    assert!(result.is_ok(), "subscribe_to_room should succeed");
}

#[tokio::test]
async fn test_broadcast_to_room() {
    let manager = ConnectionManager::new();
    let room_name = "test-room".to_string();
    let connection_id = "conn-1".to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // 订阅房间
    manager
        .subscribe_to_room(connection_id, room_name.clone(), tx)
        .await
        .unwrap();

    // 广播消息
    let message = WsMessage::new(WsMessageType::ContentCreated, None);
    let result = manager.broadcast_to_room(&room_name, message).await;

    assert!(result.is_ok(), "broadcast_to_room should succeed");
    assert_eq!(result.unwrap(), 1, "should broadcast to 1 connection");

    // 验证消息已发送
    let received: Option<WsMessage> = rx.recv().await;
    assert!(received.is_some(), "should receive message");
}

#[tokio::test]
async fn test_disconnect() {
    let manager = ConnectionManager::new();
    let room_name = "test-room".to_string();
    let connection_id = "conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel::<WsMessage>();

    // 订阅房间
    manager
        .subscribe_to_room(connection_id.clone(), room_name.clone(), tx)
        .await
        .unwrap();

    // 断开连接
    manager.disconnect(&connection_id).await;

    // 验证断开后无法广播
    let message = WsMessage::new(WsMessageType::ContentCreated, None);
    let result = manager.broadcast_to_room(&room_name, message).await;

    assert!(result.is_ok(), "broadcast_to_room should succeed");
    assert_eq!(
        result.unwrap(),
        0,
        "should broadcast to 0 connections after disconnect"
    );
}

#[tokio::test]
async fn test_multiple_subscribers_same_room() {
    let manager = ConnectionManager::new();
    let room_name = "test-room".to_string();

    // 创建多个连接
    let (tx1, mut rx1) = mpsc::unbounded_channel::<WsMessage>();
    let (tx2, mut rx2) = mpsc::unbounded_channel::<WsMessage>();
    let (tx3, mut rx3) = mpsc::unbounded_channel::<WsMessage>();

    manager
        .subscribe_to_room("conn-1".to_string(), room_name.clone(), tx1)
        .await
        .unwrap();
    manager
        .subscribe_to_room("conn-2".to_string(), room_name.clone(), tx2)
        .await
        .unwrap();
    manager
        .subscribe_to_room("conn-3".to_string(), room_name.clone(), tx3)
        .await
        .unwrap();

    // 广播消息
    let message = WsMessage::new(WsMessageType::ContentCreated, None);
    let result = manager.broadcast_to_room(&room_name, message).await;

    assert!(result.is_ok(), "broadcast_to_room should succeed");
    assert_eq!(result.unwrap(), 3, "should broadcast to 3 connections");

    // 验证所有连接都收到消息
    let msg1: Option<WsMessage> = rx1.recv().await;
    let msg2: Option<WsMessage> = rx2.recv().await;
    let msg3: Option<WsMessage> = rx3.recv().await;

    assert!(msg1.is_some(), "conn-1 should receive message");
    assert!(msg2.is_some(), "conn-2 should receive message");
    assert!(msg3.is_some(), "conn-3 should receive message");
}

// ============================================================================
// Broadcaster 测试
// ============================================================================

fn create_test_content() -> RoomContent {
    RoomContent {
        id: Some(1),
        room_id: 1,
        content_type: ContentType::Text,
        text: Some("test content".to_string()),
        url: None,
        path: None,
        file_name: None,
        size: None,
        mime_type: None,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    }
}

#[tokio::test]
async fn test_broadcaster_content_created() {
    let manager = Arc::new(ConnectionManager::new());
    let broadcaster = Broadcaster::new(manager.clone());
    let room_name = "test-room".to_string();
    let connection_id = "conn-1".to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // 订阅房间
    manager
        .subscribe_to_room(connection_id, room_name.clone(), tx)
        .await
        .unwrap();

    // 广播内容创建事件
    let content = create_test_content();
    broadcaster
        .broadcast_content_created(&room_name, &content)
        .await
        .unwrap();

    // 验证收到消息
    let received: Option<WsMessage> = rx.recv().await;
    assert!(received.is_some(), "should receive message");
    let msg = received.unwrap();
    assert_eq!(msg.message_type, WsMessageType::ContentCreated);
    assert!(msg.payload.is_some(), "message should have payload");
}

#[tokio::test]
async fn test_broadcaster_user_joined() {
    let manager = Arc::new(ConnectionManager::new());
    let broadcaster = Broadcaster::new(manager.clone());
    let room_name = "test-room".to_string();
    let connection_id = "conn-1".to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    manager
        .subscribe_to_room(connection_id, room_name.clone(), tx)
        .await
        .unwrap();

    broadcaster
        .broadcast_user_joined(&room_name, "user-1")
        .await
        .unwrap();

    let received: Option<WsMessage> = rx.recv().await;
    assert!(received.is_some(), "should receive message");
    let msg = received.unwrap();
    assert_eq!(msg.message_type, WsMessageType::UserJoined);
}

#[tokio::test]
async fn test_broadcaster_user_left() {
    let manager = Arc::new(ConnectionManager::new());
    let broadcaster = Broadcaster::new(manager.clone());
    let room_name = "test-room".to_string();
    let connection_id = "conn-1".to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    manager
        .subscribe_to_room(connection_id, room_name.clone(), tx)
        .await
        .unwrap();

    broadcaster
        .broadcast_user_left(&room_name, "user-1")
        .await
        .unwrap();

    let received: Option<WsMessage> = rx.recv().await;
    assert!(received.is_some(), "should receive message");
    let msg = received.unwrap();
    assert_eq!(msg.message_type, WsMessageType::UserLeft);
}

#[tokio::test]
async fn test_broadcaster_room_update() {
    let manager = Arc::new(ConnectionManager::new());
    let broadcaster = Broadcaster::new(manager.clone());
    let room_name = "test-room".to_string();
    let connection_id = "conn-1".to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    manager
        .subscribe_to_room(connection_id, room_name.clone(), tx)
        .await
        .unwrap();

    let room_info = RoomInfo {
        id: 1,
        name: room_name.clone(),
        slug: "test-room".to_string(),
        max_size: 1024 * 1024 * 100,
        current_size: 0,
        max_times_entered: 100,
        current_times_entered: 1,
    };

    broadcaster
        .broadcast_room_update(&room_name, &room_info)
        .await
        .unwrap();

    let received: Option<WsMessage> = rx.recv().await;
    assert!(received.is_some(), "should receive message");
    let msg = received.unwrap();
    assert_eq!(msg.message_type, WsMessageType::RoomUpdate);
}

// ============================================================================
// MessageHandler 测试
// ============================================================================

#[tokio::test]
async fn test_ws_message_serialization() {
    let message = WsMessage::new(WsMessageType::Ping, None);

    let json = serde_json::to_string(&message);
    assert!(json.is_ok(), "should serialize message");

    let deserialized: Result<WsMessage, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok(), "should deserialize message");
}

#[tokio::test]
async fn test_ws_message_with_payload() {
    let payload = serde_json::json!({
        "room": "test-room",
        "user": "test-user"
    });

    let message = WsMessage::new(WsMessageType::UserJoined, Some(payload));

    let json = serde_json::to_string(&message);
    assert!(json.is_ok(), "should serialize message with payload");

    let deserialized: Result<WsMessage, _> = serde_json::from_str(&json.unwrap());
    assert!(
        deserialized.is_ok(),
        "should deserialize message with payload"
    );
    let msg = deserialized.unwrap();
    assert!(
        msg.payload.is_some(),
        "deserialized message should have payload"
    );
}

#[tokio::test]
async fn test_room_info_serialization() {
    let room_info = RoomInfo {
        id: 1,
        name: "test-room".to_string(),
        slug: "test-room".to_string(),
        max_size: 1024 * 1024 * 100,
        current_size: 0,
        max_times_entered: 100,
        current_times_entered: 1,
    };

    let json = serde_json::to_string(&room_info);
    assert!(json.is_ok(), "should serialize RoomInfo");

    let deserialized: Result<RoomInfo, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok(), "should deserialize RoomInfo");
    let info = deserialized.unwrap();
    assert_eq!(info.name, "test-room");
    assert_eq!(info.max_size, 1024 * 1024 * 100);
}

#[tokio::test]
async fn test_ws_error_display() {
    let error = WsError::InvalidToken("bad token".to_string());
    let display = format!("{}", error);
    assert!(
        display.contains("Invalid token"),
        "error display should contain type"
    );
}

#[tokio::test]
async fn test_ws_error_serialization() {
    let error = WsError::RoomNotFound;

    let json = serde_json::to_string(&error);
    assert!(json.is_ok(), "should serialize WsError");

    let deserialized: Result<WsError, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok(), "should deserialize WsError");
    let err = deserialized.unwrap();
    matches!(err, WsError::RoomNotFound);
}

#[tokio::test]
async fn test_ws_message_new() {
    let message = WsMessage::new(WsMessageType::Pong, None);
    assert_eq!(message.message_type, WsMessageType::Pong);
    assert!(message.payload.is_none());
    assert!(message.timestamp > 0);
}

#[tokio::test]
async fn test_ws_message_error() {
    let message = WsMessage::error("test error");
    assert_eq!(message.message_type, WsMessageType::Error);
    assert!(message.payload.is_some());

    if let Some(payload) = message.payload {
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("error"), "payload should contain error field");
    }
}
