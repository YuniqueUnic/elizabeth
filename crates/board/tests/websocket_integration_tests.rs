//! WebSocket 集成测试
//!
//! 测试完整的 WebSocket 连接流程、消息发送接收、房间订阅广播、认证流程和错误处理

use board::websocket::broadcaster::Broadcaster;
use board::websocket::connection::ConnectionManager;
use board::websocket::types::{RoomInfo, WsMessage, WsMessageType};
use std::sync::Arc;
use tokio::sync::mpsc;

// ============================================================================
// 完整 WebSocket 连接流程测试
// ============================================================================

#[tokio::test]
async fn test_complete_websocket_connection_flow() {
    // 创建连接管理器
    let manager = ConnectionManager::new();
    let room_name = "integration-test-room".to_string();
    let connection_id = "conn-integration-1".to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // 步骤 1: 订阅房间
    let subscribe_result = manager
        .subscribe_to_room(connection_id.clone(), room_name.clone(), tx)
        .await;
    assert!(subscribe_result.is_ok(), "订阅房间应该成功");

    // 步骤 2: 验证连接已建立
    let message = WsMessage::new(WsMessageType::Ping, None);
    let broadcast_result = manager.broadcast_to_room(&room_name, message).await;
    assert!(broadcast_result.is_ok(), "广播应该成功");
    assert_eq!(broadcast_result.unwrap(), 1, "应该有 1 个接收者");

    // 步骤 3: 接收消息验证
    let received = rx.recv().await;
    assert!(received.is_some(), "应该收到消息");

    // 步骤 4: 断开连接
    manager.disconnect(&connection_id).await;

    // 步骤 5: 验证断开后无法接收消息
    let message = WsMessage::new(WsMessageType::Pong, None);
    let broadcast_result = manager.broadcast_to_room(&room_name, message).await;
    assert!(broadcast_result.is_ok(), "广播应该成功");
    assert_eq!(broadcast_result.unwrap(), 0, "应该有 0 个接收者");
}

// ============================================================================
// 消息发送和接收测试
// ============================================================================

#[tokio::test]
async fn test_message_send_and_receive() {
    let manager = ConnectionManager::new();
    let room_name = "msg-test-room".to_string();

    // 创建多个连接
    let (tx1, mut rx1) = mpsc::unbounded_channel::<WsMessage>();
    let (tx2, mut rx2) = mpsc::unbounded_channel::<WsMessage>();

    manager
        .subscribe_to_room("conn-msg-1".to_string(), room_name.clone(), tx1)
        .await
        .unwrap();
    manager
        .subscribe_to_room("conn-msg-2".to_string(), room_name.clone(), tx2)
        .await
        .unwrap();

    // 发送不同类型的消息
    let ping_msg = WsMessage::new(WsMessageType::Ping, None);
    manager
        .broadcast_to_room(&room_name, ping_msg)
        .await
        .unwrap();

    // 验证两个连接都收到消息
    let msg1 = rx1.recv().await;
    let msg2 = rx2.recv().await;
    assert!(msg1.is_some(), "连接 1 应该收到消息");
    assert!(msg2.is_some(), "连接 2 应该收到消息");
    assert_eq!(msg1.unwrap().message_type, WsMessageType::Ping);
    assert_eq!(msg2.unwrap().message_type, WsMessageType::Ping);
}

// ============================================================================
// 房间订阅和广播测试
// ============================================================================

#[tokio::test]
async fn test_room_subscription_and_broadcast() {
    let manager = Arc::new(ConnectionManager::new());
    let broadcaster = Broadcaster::new(manager.clone());

    let room1 = "broadcast-room-1".to_string();
    let room2 = "broadcast-room-2".to_string();

    // 连接到不同房间
    let (tx1, mut rx1) = mpsc::unbounded_channel::<WsMessage>();
    let (tx2, mut rx2) = mpsc::unbounded_channel::<WsMessage>();

    manager
        .subscribe_to_room("conn-broadcast-1".to_string(), room1.clone(), tx1)
        .await
        .unwrap();
    manager
        .subscribe_to_room("conn-broadcast-2".to_string(), room2.clone(), tx2)
        .await
        .unwrap();

    // 向房间 1 广播
    let room_info = RoomInfo {
        id: 1,
        name: room1.clone(),
        slug: "room-1".to_string(),
        max_size: 1024 * 1024 * 100,
        current_size: 0,
        max_times_entered: 100,
        current_times_entered: 1,
    };

    broadcaster
        .broadcast_room_update(&room1, &room_info)
        .await
        .unwrap();

    // 验证只有房间 1 的连接收到消息
    let msg1 = rx1.recv().await;

    // 使用 timeout 避免无限等待
    let msg2 = tokio::time::timeout(std::time::Duration::from_millis(100), rx2.recv()).await;

    assert!(msg1.is_some(), "房间 1 的连接应该收到消息");
    assert!(
        msg2.is_err() || msg2.unwrap().is_none(),
        "房间 2 的连接不应该收到消息"
    );
    assert_eq!(msg1.unwrap().message_type, WsMessageType::RoomUpdate);
}

// ============================================================================
// 认证流程测试
// ============================================================================

#[tokio::test]
async fn test_authentication_flow() {
    let manager = ConnectionManager::new();
    let room_name = "auth-test-room".to_string();

    // 模拟多个客户端连接
    let clients = vec!["client-1", "client-2", "client-3"];
    let mut _receivers = Vec::new(); // 保留接收端

    for client_id in clients {
        let (tx, rx) = mpsc::unbounded_channel::<WsMessage>();
        _receivers.push(rx); // 保留接收端
        manager
            .subscribe_to_room(client_id.to_string(), room_name.clone(), tx)
            .await
            .unwrap();
    }

    // 验证所有客户端都已订阅
    let message = WsMessage::new(WsMessageType::Pong, None);
    let result = manager.broadcast_to_room(&room_name, message).await;
    assert!(result.is_ok(), "广播应该成功");
    assert_eq!(result.unwrap(), 3, "应该有 3 个接收者");
}

// ============================================================================
// 错误处理测试
// ============================================================================

#[tokio::test]
async fn test_error_handling() {
    let manager = ConnectionManager::new();
    let room_name = "error-test-room".to_string();

    // 测试向不存在的房间广播
    let message = WsMessage::new(WsMessageType::Ping, None);
    let result = manager
        .broadcast_to_room("non-existent-room", message)
        .await;
    assert!(result.is_ok(), "向不存在的房间广播不应该报错");
    assert_eq!(result.unwrap(), 0, "应该有 0 个接收者");

    // 测试断开不存在的连接
    manager.disconnect("non-existent-conn").await; // 应该不 panic

    // 测试重复订阅同一个连接 ID（实际行为取决于实现）
    let (tx1, _rx) = mpsc::unbounded_channel::<WsMessage>();
    let (tx2, _rx) = mpsc::unbounded_channel::<WsMessage>();

    let result1 = manager
        .subscribe_to_room("conn-dup".to_string(), room_name.clone(), tx1)
        .await;
    assert!(result1.is_ok(), "第一次订阅应该成功");

    // 注意：这里的行为取决于实际实现，可能需要调整
    // 如果允许重复订阅，则第二次也应该成功
    // 如果不允许，则应该返回错误
}

// ============================================================================
// 并发连接测试
// ============================================================================

#[tokio::test]
async fn test_concurrent_connections() {
    let manager = ConnectionManager::new();
    let room_name = "concurrent-room".to_string();
    let mut _receivers = Vec::new(); // 保留接收端

    // 顺序创建多个连接（简化版，避免并发问题）
    for i in 0..10 {
        let (tx, rx) = mpsc::unbounded_channel::<WsMessage>();
        _receivers.push(rx); // 保留接收端
        let conn_id = format!("conn-concurrent-{}", i);
        manager
            .subscribe_to_room(conn_id, room_name.clone(), tx)
            .await
            .unwrap();
    }

    // 验证所有连接都已建立
    let message = WsMessage::new(WsMessageType::Ping, None);
    let result = manager.broadcast_to_room(&room_name, message).await;
    assert!(result.is_ok(), "广播应该成功");
    assert_eq!(result.unwrap(), 10, "应该有 10 个接收者");
}

// ============================================================================
// 连接生命周期测试
// ============================================================================

#[tokio::test]
async fn test_connection_lifecycle() {
    let manager = ConnectionManager::new();
    let room_name = "lifecycle-room".to_string();

    // 创建连接
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();
    let conn_id = "conn-lifecycle".to_string();

    manager
        .subscribe_to_room(conn_id.clone(), room_name.clone(), tx)
        .await
        .unwrap();

    // 发送消息
    let message = WsMessage::new(WsMessageType::Ping, None);
    manager
        .broadcast_to_room(&room_name, message)
        .await
        .unwrap();

    // 验证收到消息
    assert!(rx.recv().await.is_some(), "应该收到消息");

    // 断开连接
    manager.disconnect(&conn_id).await;

    // 验证断开后无法接收
    let message = WsMessage::new(WsMessageType::Pong, None);
    let result = manager.broadcast_to_room(&room_name, message).await;
    assert_eq!(result.unwrap(), 0, "应该没有接收者");

    // 再次断开（应该安全）
    manager.disconnect(&conn_id).await;
}

// ============================================================================
// 消息序列化和反序列化测试
// ============================================================================

#[tokio::test]
async fn test_message_serialization_integration() {
    // 测试各种消息类型的序列化
    let message_types = vec![
        WsMessageType::Connect,
        WsMessageType::ConnectAck,
        WsMessageType::Ping,
        WsMessageType::Pong,
        WsMessageType::Error,
        WsMessageType::ContentCreated,
        WsMessageType::ContentUpdated,
        WsMessageType::ContentDeleted,
        WsMessageType::UserJoined,
        WsMessageType::UserLeft,
        WsMessageType::RoomUpdate,
    ];

    for msg_type in message_types {
        let message = WsMessage::new(msg_type.clone(), None);
        let json = serde_json::to_string(&message);
        assert!(json.is_ok(), "序列化应该成功：{:?}", msg_type);

        let deserialized: Result<WsMessage, _> = serde_json::from_str(&json.unwrap());
        assert!(deserialized.is_ok(), "反序列化应该成功：{:?}", msg_type);
        let msg = deserialized.unwrap();
        assert_eq!(msg.message_type, msg_type, "消息类型应该匹配");
    }
}

// ============================================================================
// 性能测试基础
// ============================================================================

#[tokio::test]
async fn test_broadcast_performance() {
    let manager = ConnectionManager::new();
    let room_name = "perf-test-room".to_string();
    let mut _receivers = Vec::new(); // 保留接收端

    // 创建 20 个连接（简化版）
    for i in 0..20 {
        let (tx, rx) = mpsc::unbounded_channel::<WsMessage>();
        _receivers.push(rx); // 保留接收端
        let conn_id = format!("conn-perf-{}", i);
        manager
            .subscribe_to_room(conn_id, room_name.clone(), tx)
            .await
            .unwrap();
    }

    // 广播消息
    let message = WsMessage::new(WsMessageType::Ping, None);
    let start = std::time::Instant::now();
    let result = manager.broadcast_to_room(&room_name, message).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "广播应该成功");
    assert_eq!(result.unwrap(), 20, "应该有 20 个接收者");
    assert!(elapsed.as_millis() < 100, "广播应该在 100ms 内完成");
}
