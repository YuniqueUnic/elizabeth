//! WebSocket 模块单元测试
//!
//! 测试 ConnectionManager、Broadcaster 和心跳任务

use board::models::room::content::{ContentType, RoomContent};
use board::websocket::broadcaster::Broadcaster;
use board::websocket::connection::ConnectionManager;
use board::websocket::server::WsServer;
use board::websocket::types::{RoomInfo, RoomUpdateReason, WsError, WsMessage, WsMessageType};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

// ============================================================================
// ConnectionManager 测试
// ============================================================================

#[tokio::test]
async fn test_connection_manager_new() {
    let _manager = ConnectionManager::new();
    // 测试创建成功 - 如果能创建到这行，说明没有 panic
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
        created_by_jti: None,
        hidden: false,
        room_id: 1,
        content_type: ContentType::Text,
        text: Some("test content".to_string()),
        url: None,
        path: None,
        hash: None,
        file_name: None,
        size: None,
        mime_type: None,
        sequence_number: 0,
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
    let payload = msg.payload.expect("message should have payload");
    assert_eq!(payload["content_id"].as_i64(), content.id);
    assert_eq!(
        payload["sequence_number"].as_i64(),
        Some(i64::from(content.sequence_number))
    );
    assert_eq!(payload["text"].as_str(), content.text.as_deref());
    assert!(payload["created_at"].is_string());
    assert!(payload["updated_at"].is_string());
}

#[tokio::test]
async fn test_broadcaster_content_deleted_includes_content_metadata() {
    let manager = Arc::new(ConnectionManager::new());
    let broadcaster = Broadcaster::new(manager.clone());
    let room_name = "test-room".to_string();
    let connection_id = "conn-1".to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    manager
        .subscribe_to_room(connection_id, room_name.clone(), tx)
        .await
        .unwrap();

    let content = create_test_content();
    broadcaster
        .broadcast_content_deleted(&room_name, &content)
        .await
        .unwrap();

    let received: Option<WsMessage> = rx.recv().await;
    assert!(received.is_some(), "should receive message");
    let msg = received.unwrap();
    assert_eq!(msg.message_type, WsMessageType::ContentDeleted);

    let payload = msg.payload.expect("message should have payload");
    assert_eq!(payload["content_id"].as_i64(), Some(1));
    assert_eq!(payload["room_name"].as_str(), Some(room_name.as_str()));
    assert_eq!(payload["content_type"]["type"].as_str(), Some("text"));
    assert_eq!(payload["text"].as_str(), Some("test content"));
    assert_eq!(payload["sequence_number"].as_i64(), Some(0));
    assert!(payload["created_at"].is_string());
    assert!(payload["updated_at"].is_string());
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
        .broadcast_room_update(&room_name, &room_info, RoomUpdateReason::SettingsChanged)
        .await
        .unwrap();

    let received: Option<WsMessage> = rx.recv().await;
    assert!(received.is_some(), "should receive message");
    let msg = received.unwrap();
    assert_eq!(msg.message_type, WsMessageType::RoomUpdate);
    assert_eq!(
        msg.payload
            .as_ref()
            .and_then(|payload| payload.get("reason"))
            .and_then(|reason| reason.as_str()),
        Some("settings_changed")
    );
}

// ============================================================================
// WsMessage / WsError 测试
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

    let message = WsMessage::new(WsMessageType::ContentCreated, Some(payload));

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

// ============================================================================
// 心跳测试
// ============================================================================

#[tokio::test]
async fn test_heartbeat_keeps_pushing_ping_frames() {
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();
    let task = tokio::spawn(WsServer::send_heartbeats(tx, Duration::from_millis(20)));

    // 连续三帧都是 PING，说明是周期推送而不是只发一次
    for round in 1..=3 {
        let received = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap_or_else(|_| panic!("heartbeat should push frame {}", round))
            .expect("channel should stay open while the connection lives");
        assert_eq!(received.message_type, WsMessageType::Ping);
        assert!(received.payload.is_none(), "PING 不需要载荷");
    }

    task.abort();
}

#[tokio::test]
async fn test_heartbeat_stops_after_connection_closes() {
    let (tx, rx) = mpsc::unbounded_channel::<WsMessage>();
    let task = tokio::spawn(WsServer::send_heartbeats(tx, Duration::from_millis(10)));

    // 连接结束 = 接收端被丢弃
    drop(rx);

    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("heartbeat should exit once the connection closes")
        .expect("heartbeat task should not panic");
}
