# Elizabeth WebSocket 实时通信指南

本文档详细介绍 Elizabeth 平台的 WebSocket
实时通信功能，包括连接建立、消息协议、事件类型和最佳实践。

## 目录

- [概述](#概述)
- [连接建立](#连接建立)
- [消息协议](#消息协议)
- [事件类型](#事件类型)
- [客户端实现](#客户端实现)
- [错误处理](#错误处理)
- [最佳实践](#最佳实践)
- [完整示例](#完整示例)

## 概述

### WebSocket 服务端点

```
ws://localhost:4092/api/v1/ws       # Docker Compose（推荐）
ws://127.0.0.1:4092/api/v1/ws       # 后端直连（开发/调试）
wss://your-domain.com/api/v1/ws     # 生产环境（使用 WSS 加密）
```

### 核心功能

- **实时文件同步**: 房间内文件上传、更新、删除实时广播
- **用户状态感知**: 用户加入/离开房间事件通知
- **房间信息更新**: 房间容量、权限等设置变更实时推送
- **心跳检测**: 自动保持连接活跃，检测连接状态

### 架构设计

```
┌─────────────────────────────────────────────────────────┐
│                   WebSocket Server                      │
│  入口：/api/v1/ws                                       │
└────────────────────┬────────────────────────────────────┘
                     │
         ┌───────────▼───────────┐
         │  Connection Manager   │
         │  • 房间订阅管理        │
         │  • 连接生命周期        │
         │  • 广播消息分发        │
         └───────────┬───────────┘
                     │
         ┌───────────▼───────────┐
         │   Message Handler     │
         │  • 消息解析与验证      │
         │  • Token 认证         │
         │  • 业务逻辑处理        │
         └───────────────────────┘
```

---

## 连接建立

### 步骤 1: 获取房间 Token

在建立 WebSocket 连接前，必须先获取有效的房间访问 Token。

```bash
curl -X POST "http://localhost:4092/api/v1/rooms/my-room/tokens" \
  -H "Content-Type: application/json" \
  -d '{
    "password": "room-password",
    "with_refresh_token": true
  }'
```

响应包含 `token` 字段，将用于 WebSocket 认证。

### 步骤 2: 建立 WebSocket 连接

```javascript
const ws = new WebSocket("ws://localhost:4092/api/v1/ws");

ws.onopen = () => {
  console.log("WebSocket 连接已建立");

  // 发送连接请求
  const connectMessage = {
    message_type: "connect",
    payload: {
      room_name: "my-room",
      token: "eyJhbGc...", // 步骤 1 获取的 Token
    },
    timestamp: Date.now(),
  };

  ws.send(JSON.stringify(connectMessage));
};
```

### 步骤 3: 接收连接确认

```javascript
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  if (message.message_type === "connect_ack") {
    const { success, message: msg, room_info } = message.payload;

    if (success) {
      console.log("成功加入房间：", msg);
      console.log("房间信息：", room_info);
      // 连接成功，可以开始接收和发送消息
    } else {
      console.error("连接失败：", msg);
      ws.close();
    }
  }
};
```

### 连接流程图

```
客户端                           服务端
  │                                │
  ├──── WebSocket 握手 ────────────▶│
  │                                │
  │◀───── 101 Switching Protocols ─┤
  │                                │
  ├──── CONNECT 消息 ──────────────▶│
  │     {room_name, token}         │
  │                                ├─ 验证 Token
  │                                ├─ 检查房间状态
  │                                ├─ 注册连接
  │                                │
  │◀───── CONNECT_ACK ─────────────┤
  │     {success, room_info}       │
  │                                │
  ├──── 开始接收房间事件 ───────────▶│
  │                                │
```

---

## 消息协议

### 消息结构

所有 WebSocket 消息使用统一的 JSON 格式：

```typescript
interface WsMessage {
  message_type: WsMessageType; // 消息类型
  payload?: any; // 消息载荷（可选）
  timestamp: number; // Unix 时间戳（毫秒）
}
```

### 消息类型枚举

```typescript
enum WsMessageType {
  // 连接管理
  Connect = "connect", // 连接请求
  ConnectAck = "connect_ack", // 连接确认

  // 心跳
  Ping = "ping", // 心跳请求
  Pong = "pong", // 心跳响应

  // 内容事件
  ContentCreated = "content_created", // 文件/消息创建
  ContentUpdated = "content_updated", // 内容更新
  ContentDeleted = "content_deleted", // 内容删除

  // 用户事件
  UserJoined = "user_joined", // 用户加入房间
  UserLeft = "user_left", // 用户离开房间

  // 房间事件
  RoomUpdate = "room_update", // 房间信息更新

  // 错误
  Error = "error", // 错误消息
}
```

---

## 事件类型

### 1. 连接事件

#### CONNECT (客户端 → 服务端)

客户端发送连接请求，加入指定房间。

**消息格式：**

```json
{
  "message_type": "connect",
  "payload": {
    "room_name": "my-room",
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  },
  "timestamp": 1737368400000
}
```

**字段说明：**

- `room_name`: 要加入的房间名称或 slug
- `token`: 有效的房间访问 Token（通过 REST API 获取）

---

#### CONNECT_ACK (服务端 → 客户端)

服务端响应连接请求，确认是否成功加入房间。

**消息格式：**

```json
{
  "message_type": "connect_ack",
  "payload": {
    "success": true,
    "message": "Connected successfully",
    "room_info": {
      "id": 1,
      "name": "my-room",
      "slug": "my-room-a1b2c3",
      "max_size": 10737418240,
      "current_size": 2048576,
      "max_times_entered": 9223372036854775807,
      "current_times_entered": 5
    }
  },
  "timestamp": 1737368400500
}
```

**字段说明：**

- `success`: 是否成功连接
- `message`: 连接结果消息
- `room_info`: 房间信息（成功时提供）

**错误响应示例：**

```json
{
  "message_type": "connect_ack",
  "payload": {
    "success": false,
    "message": "Token is invalid or expired"
  },
  "timestamp": 1737368400500
}
```

---

### 2. 心跳事件

#### PING (客户端 ↔ 服务端)

用于检测连接是否活跃。客户端和服务端都可以发起。

**消息格式：**

```json
{
  "message_type": "ping",
  "payload": null,
  "timestamp": 1737368460000
}
```

**建议：** 客户端每 30 秒发送一次 PING，服务端每 60 秒发送一次。

---

#### PONG (响应方 → 发起方)

心跳响应消息。

**消息格式：**

```json
{
  "message_type": "pong",
  "payload": null,
  "timestamp": 1737368460100
}
```

**使用场景：**

- 检测连接是否断开
- 测量网络延迟（timestamp 差值）
- 保持连接活跃，防止超时

---

### 3. 内容事件

#### CONTENT_CREATED (服务端 → 客户端)

房间内有新文件或消息创建时广播。

**消息格式：**

```json
{
  "message_type": "content_created",
  "payload": {
    "content_id": 42,
    "room_name": "my-room",
    "content_type": "file",
    "text": null,
    "file_name": "document.pdf",
    "file_size": 2048576,
    "created_at": "2026-01-20T10:50:00"
  },
  "timestamp": 1737368460000
}
```

**字段说明：**

- `content_id`: 内容唯一 ID
- `content_type`: 内容类型 ("file" 或 "message")
- `file_name`: 文件名（仅文件类型）
- `file_size`: 文件大小（字节，仅文件类型）
- `text`: 消息文本（仅消息类型）

**客户端处理示例：**

```javascript
if (message.message_type === "content_created") {
  const { content_type, file_name, text } = message.payload;

  if (content_type === "file") {
    console.log(`新文件上传：${file_name}`);
    // 刷新文件列表 UI
    refreshFileList();
  } else if (content_type === "message") {
    console.log(`新消息：${text}`);
    // 添加消息到聊天界面
    addMessageToChat(text);
  }
}
```

---

#### CONTENT_UPDATED (服务端 → 客户端)

房间内容被更新时广播（如文件重命名、消息编辑）。

**消息格式：**

```json
{
  "message_type": "content_updated",
  "payload": {
    "content_id": 42,
    "room_name": "my-room",
    "content_type": "message",
    "text": "编辑后的消息内容",
    "updated_at": "2026-01-20T11:00:00"
  },
  "timestamp": 1737369000000
}
```

---

#### CONTENT_DELETED (服务端 → 客户端)

房间内容被删除时广播。

**消息格式：**

```json
{
  "message_type": "content_deleted",
  "payload": {
    "content_id": 42,
    "room_name": "my-room",
    "content_type": "file",
    "file_name": "document.pdf",
    "deleted_at": "2026-01-20T11:10:00"
  },
  "timestamp": 1737369600000
}
```

**客户端处理示例：**

```javascript
if (message.message_type === "content_deleted") {
  const { content_id } = message.payload;

  // 从 UI 中移除对应内容
  removeContentFromUI(content_id);
}
```

---

### 4. 用户事件

#### USER_JOINED (服务端 → 客户端)

有新用户加入房间时广播。

**消息格式：**

```json
{
  "message_type": "user_joined",
  "payload": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "room_name": "my-room",
    "joined_at": "2026-01-20T11:20:00"
  },
  "timestamp": 1737370200000
}
```

**字段说明：**

- `user_id`: 用户连接 ID（UUID）
- `room_name`: 房间名称

**客户端处理示例：**

```javascript
if (message.message_type === "user_joined") {
  const { user_id } = message.payload;

  console.log(`用户 ${user_id} 加入了房间`);

  // 更新在线用户列表
  addUserToOnlineList(user_id);

  // 显示通知
  showNotification(`有新成员加入房间`);
}
```

---

#### USER_LEFT (服务端 → 客户端)

用户离开房间时广播。

**消息格式：**

```json
{
  "message_type": "user_left",
  "payload": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "room_name": "my-room",
    "left_at": "2026-01-20T11:30:00"
  },
  "timestamp": 1737370800000
}
```

**客户端处理示例：**

```javascript
if (message.message_type === "user_left") {
  const { user_id } = message.payload;

  console.log(`用户 ${user_id} 离开了房间`);

  // 从在线用户列表移除
  removeUserFromOnlineList(user_id);
}
```

---

### 5. 房间事件

#### ROOM_UPDATE (服务端 → 客户端)

房间设置或状态更新时广播（如容量限制、权限变更）。

**消息格式：**

```json
{
  "message_type": "room_update",
  "payload": {
    "room_name": "my-room",
    "room_info": {
      "id": 1,
      "name": "my-room",
      "slug": "my-room-a1b2c3",
      "max_size": 5368709120,
      "current_size": 2048576,
      "max_times_entered": 100,
      "current_times_entered": 5
    }
  },
  "timestamp": 1737371400000
}
```

**触发场景：**

- 房间容量限制变更
- 房间权限更新
- 房间过期时间修改
- 房间最大进入次数变更

**客户端处理示例：**

```javascript
if (message.message_type === "room_update") {
  const { room_info } = message.payload;

  console.log("房间信息已更新：", room_info);

  // 更新 UI 中的房间信息
  updateRoomInfoUI(room_info);

  // 检查容量限制
  const usagePercent = (room_info.current_size / room_info.max_size) * 100;
  if (usagePercent > 90) {
    showWarning("房间容量即将用尽");
  }
}
```

---

### 6. 错误事件

#### ERROR (服务端 → 客户端)

服务端发生错误时发送。

**消息格式：**

```json
{
  "message_type": "error",
  "payload": {
    "error": "Token is invalid or expired"
  },
  "timestamp": 1737372000000
}
```

**常见错误类型：**

- `Invalid token`: Token 无效或已过期
- `Room not found`: 房间不存在
- `Permission denied`: 权限不足
- `Invalid message format`: 消息格式错误
- `Internal error`: 服务器内部错误

**客户端处理示例：**

```javascript
if (message.message_type === "error") {
  const { error } = message.payload;

  console.error("WebSocket 错误：", error);

  // 根据错误类型处理
  if (error.includes("Token")) {
    // Token 相关错误，尝试重新获取 Token
    reconnectWithNewToken();
  } else if (error.includes("Permission")) {
    // 权限错误，提示用户
    showError("权限不足，无法执行该操作");
  } else {
    // 其他错误
    showError(`发生错误：${error}`);
  }
}
```

---

## 客户端实现

### JavaScript/TypeScript 实现

#### 基础连接类

```typescript
class ElizabethWebSocket {
  private ws: WebSocket | null = null;
  private roomName: string;
  private token: string;
  private pingInterval: NodeJS.Timeout | null = null;
  private reconnectTimeout: NodeJS.Timeout | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;

  constructor(roomName: string, token: string) {
    this.roomName = roomName;
    this.token = token;
  }

  connect() {
    const wsUrl = "ws://localhost:4092/api/v1/ws";
    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = this.handleOpen.bind(this);
    this.ws.onmessage = this.handleMessage.bind(this);
    this.ws.onerror = this.handleError.bind(this);
    this.ws.onclose = this.handleClose.bind(this);
  }

  private handleOpen() {
    console.log("WebSocket 连接已建立");
    this.reconnectAttempts = 0;

    // 发送连接请求
    this.sendMessage({
      message_type: "connect",
      payload: {
        room_name: this.roomName,
        token: this.token,
      },
      timestamp: Date.now(),
    });

    // 启动心跳
    this.startHeartbeat();
  }

  private handleMessage(event: MessageEvent) {
    const message = JSON.parse(event.data);
    console.log("收到消息：", message);

    switch (message.message_type) {
      case "connect_ack":
        this.handleConnectAck(message);
        break;
      case "ping":
        this.handlePing();
        break;
      case "pong":
        console.log("收到 PONG");
        break;
      case "content_created":
        this.handleContentCreated(message);
        break;
      case "content_updated":
        this.handleContentUpdated(message);
        break;
      case "content_deleted":
        this.handleContentDeleted(message);
        break;
      case "user_joined":
        this.handleUserJoined(message);
        break;
      case "user_left":
        this.handleUserLeft(message);
        break;
      case "room_update":
        this.handleRoomUpdate(message);
        break;
      case "error":
        this.handleServerError(message);
        break;
    }
  }

  private handleConnectAck(message: any) {
    const { success, message: msg, room_info } = message.payload;

    if (success) {
      console.log("成功加入房间：", msg);
      console.log("房间信息：", room_info);
      // 触发连接成功事件
      this.onConnected?.(room_info);
    } else {
      console.error("连接失败：", msg);
      this.close();
    }
  }

  private handlePing() {
    // 响应服务端 PING
    this.sendMessage({
      message_type: "pong",
      payload: null,
      timestamp: Date.now(),
    });
  }

  private handleContentCreated(message: any) {
    console.log("内容创建：", message.payload);
    this.onContentCreated?.(message.payload);
  }

  private handleContentUpdated(message: any) {
    console.log("内容更新：", message.payload);
    this.onContentUpdated?.(message.payload);
  }

  private handleContentDeleted(message: any) {
    console.log("内容删除：", message.payload);
    this.onContentDeleted?.(message.payload);
  }

  private handleUserJoined(message: any) {
    console.log("用户加入：", message.payload);
    this.onUserJoined?.(message.payload);
  }

  private handleUserLeft(message: any) {
    console.log("用户离开：", message.payload);
    this.onUserLeft?.(message.payload);
  }

  private handleRoomUpdate(message: any) {
    console.log("房间更新：", message.payload);
    this.onRoomUpdate?.(message.payload);
  }

  private handleServerError(message: any) {
    console.error("服务器错误：", message.payload);
    this.onError?.(message.payload.error);
  }

  private handleError(event: Event) {
    console.error("WebSocket 错误：", event);
  }

  private handleClose(event: CloseEvent) {
    console.log("WebSocket 连接关闭：", event.code, event.reason);
    this.stopHeartbeat();

    // 尝试重连
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);

      console.log(
        `${delay}ms 后尝试重连 (${this.reconnectAttempts}/${this.maxReconnectAttempts})`,
      );

      this.reconnectTimeout = setTimeout(() => {
        this.connect();
      }, delay);
    } else {
      console.error("达到最大重连次数，放弃重连");
      this.onConnectionLost?.();
    }
  }

  private startHeartbeat() {
    // 每 30 秒发送一次 PING
    this.pingInterval = setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.sendMessage({
          message_type: "ping",
          payload: null,
          timestamp: Date.now(),
        });
      }
    }, 30000);
  }

  private stopHeartbeat() {
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }

  private sendMessage(message: any) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn("WebSocket 未连接，无法发送消息");
    }
  }

  close() {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
    }
    this.stopHeartbeat();
    this.ws?.close();
  }

  // 事件回调（由使用方设置）
  onConnected?: (roomInfo: any) => void;
  onContentCreated?: (payload: any) => void;
  onContentUpdated?: (payload: any) => void;
  onContentDeleted?: (payload: any) => void;
  onUserJoined?: (payload: any) => void;
  onUserLeft?: (payload: any) => void;
  onRoomUpdate?: (payload: any) => void;
  onError?: (error: string) => void;
  onConnectionLost?: () => void;
}
```

---

#### 使用示例

```typescript
// 1. 获取 Token
const response = await fetch(
  "http://localhost:4092/api/v1/rooms/my-room/tokens",
  {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      password: "room-password",
      with_refresh_token: true,
    }),
  },
);
const { token } = await response.json();

// 2. 创建 WebSocket 连接
const ws = new ElizabethWebSocket("my-room", token);

// 3. 设置事件监听器
ws.onConnected = (roomInfo) => {
  console.log("已连接到房间：", roomInfo);
  document.getElementById("status").textContent = "已连接";
};

ws.onContentCreated = (payload) => {
  if (payload.content_type === "file") {
    addFileToList(payload);
  } else {
    addMessageToChat(payload);
  }
};

ws.onContentDeleted = (payload) => {
  removeContentFromUI(payload.content_id);
};

ws.onUserJoined = (payload) => {
  showNotification(`用户 ${payload.user_id} 加入了房间`);
  updateOnlineUserCount(+1);
};

ws.onUserLeft = (payload) => {
  updateOnlineUserCount(-1);
};

ws.onRoomUpdate = (payload) => {
  updateRoomInfo(payload.room_info);
};

ws.onError = (error) => {
  console.error("发生错误：", error);
  showErrorNotification(error);
};

ws.onConnectionLost = () => {
  document.getElementById("status").textContent = "连接丢失";
  showReconnectButton();
};

// 4. 建立连接
ws.connect();

// 5. 页面关闭时断开连接
window.addEventListener("beforeunload", () => {
  ws.close();
});
```

---

### React Hook 实现

```typescript
import { useEffect, useRef, useState } from "react";

interface UseWebSocketOptions {
  roomName: string;
  token: string;
  onContentCreated?: (payload: any) => void;
  onContentUpdated?: (payload: any) => void;
  onContentDeleted?: (payload: any) => void;
  onUserJoined?: (payload: any) => void;
  onUserLeft?: (payload: any) => void;
  onRoomUpdate?: (payload: any) => void;
}

export function useElizabethWebSocket(options: UseWebSocketOptions) {
  const [isConnected, setIsConnected] = useState(false);
  const [roomInfo, setRoomInfo] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<ElizabethWebSocket | null>(null);

  useEffect(() => {
    const ws = new ElizabethWebSocket(options.roomName, options.token);

    ws.onConnected = (info) => {
      setIsConnected(true);
      setRoomInfo(info);
      setError(null);
    };

    ws.onContentCreated = options.onContentCreated;
    ws.onContentUpdated = options.onContentUpdated;
    ws.onContentDeleted = options.onContentDeleted;
    ws.onUserJoined = options.onUserJoined;
    ws.onUserLeft = options.onUserLeft;
    ws.onRoomUpdate = (payload) => {
      setRoomInfo(payload.room_info);
      options.onRoomUpdate?.(payload);
    };

    ws.onError = (err) => {
      setError(err);
    };

    ws.onConnectionLost = () => {
      setIsConnected(false);
      setError("连接丢失");
    };

    ws.connect();
    wsRef.current = ws;

    return () => {
      ws.close();
    };
  }, [options.roomName, options.token]);

  return {
    isConnected,
    roomInfo,
    error,
    disconnect: () => wsRef.current?.close(),
  };
}

// 使用示例
function RoomPage({ roomName, token }: { roomName: string; token: string }) {
  const { isConnected, roomInfo, error } = useElizabethWebSocket({
    roomName,
    token,
    onContentCreated: (payload) => {
      console.log("新内容：", payload);
      // 刷新文件列表
    },
    onUserJoined: (payload) => {
      console.log("用户加入：", payload);
    },
  });

  return (
    <div>
      <div>状态：{isConnected ? "已连接" : "未连接"}</div>
      {error && <div style={{ color: "red" }}>错误：{error}</div>}
      {roomInfo && (
        <div>
          <p>房间：{roomInfo.name}</p>
          <p>容量：{roomInfo.current_size} / {roomInfo.max_size}</p>
        </div>
      )}
    </div>
  );
}
```

---

## 错误处理

### 常见错误及处理

| 错误类型                    | 说明               | 处理方式              |
| --------------------------- | ------------------ | --------------------- |
| Token is invalid or expired | Token 无效或已过期 | 重新获取 Token 并重连 |
| Room not found              | 房间不存在         | 提示用户，停止重连    |
| Permission denied           | 权限不足           | 提示用户权限不足      |
| Invalid message format      | 消息格式错误       | 检查客户端代码        |
| Connection timeout          | 连接超时           | 使用指数退避重连      |
| Max connections reached     | 房间连接数已满     | 提示用户稍后重试      |

### 重连策略

```typescript
class ReconnectStrategy {
  private attempts = 0;
  private maxAttempts = 5;
  private baseDelay = 1000;
  private maxDelay = 30000;

  shouldReconnect(): boolean {
    return this.attempts < this.maxAttempts;
  }

  getDelay(): number {
    // 指数退避：1s, 2s, 4s, 8s, 16s, max 30s
    const delay = this.baseDelay * Math.pow(2, this.attempts);
    return Math.min(delay, this.maxDelay);
  }

  recordAttempt() {
    this.attempts++;
  }

  reset() {
    this.attempts = 0;
  }
}
```

---

## 最佳实践

### 1. 连接管理

**及时释放连接：**

```javascript
// ✅ 正确：页面卸载时关闭连接
useEffect(() => {
  const ws = new ElizabethWebSocket(roomName, token);
  ws.connect();

  return () => {
    ws.close(); // 清理连接
  };
}, []);

// ❌ 错误：忘记关闭连接
useEffect(() => {
  const ws = new ElizabethWebSocket(roomName, token);
  ws.connect();
  // 缺少 cleanup
}, []);
```

**避免重复连接：**

```javascript
// ✅ 正确：检查连接状态
if (!wsRef.current || wsRef.current.readyState === WebSocket.CLOSED) {
  const ws = new WebSocket(url);
  wsRef.current = ws;
}

// ❌ 错误：不检查直接创建
const ws = new WebSocket(url); // 可能创建多个连接
```

---

### 2. Token 管理

**Token 过期处理：**

```typescript
class TokenManager {
  private token: string;
  private refreshToken: string;
  private expiresAt: Date;

  async getValidToken(): Promise<string> {
    // 如果 Token 即将过期（5 分钟内），刷新它
    const fiveMinutesLater = new Date(Date.now() + 5 * 60 * 1000);

    if (this.expiresAt < fiveMinutesLater) {
      await this.refreshAccessToken();
    }

    return this.token;
  }

  private async refreshAccessToken() {
    const response = await fetch("/api/v1/auth/refresh", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refresh_token: this.refreshToken }),
    });

    const data = await response.json();
    this.token = data.access_token;
    this.refreshToken = data.refresh_token;
    this.expiresAt = new Date(data.expires_at);
  }
}
```

---

### 3. 心跳机制

**推荐配置：**

- 客户端发送间隔：30 秒
- 服务端发送间隔：60 秒
- PONG 超时：5 秒
- 超时后动作：重连

**实现示例：**

```typescript
class HeartbeatManager {
  private pingInterval: NodeJS.Timeout | null = null;
  private pongTimeout: NodeJS.Timeout | null = null;
  private lastPongTime = Date.now();

  start(ws: WebSocket) {
    this.pingInterval = setInterval(() => {
      // 检查上次 PONG 时间
      if (Date.now() - this.lastPongTime > 65000) {
        console.warn("未收到 PONG，连接可能已断开");
        ws.close();
        return;
      }

      // 发送 PING
      ws.send(JSON.stringify({
        message_type: "ping",
        payload: null,
        timestamp: Date.now(),
      }));

      // 设置 PONG 超时
      this.pongTimeout = setTimeout(() => {
        console.warn("PONG 超时");
        ws.close();
      }, 5000);
    }, 30000);
  }

  onPong() {
    this.lastPongTime = Date.now();
    if (this.pongTimeout) {
      clearTimeout(this.pongTimeout);
    }
  }

  stop() {
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
    }
    if (this.pongTimeout) {
      clearTimeout(this.pongTimeout);
    }
  }
}
```

---

### 4. 消息队列

**离线消息缓存：**

```typescript
class MessageQueue {
  private queue: any[] = [];
  private maxSize = 100;

  enqueue(message: any) {
    this.queue.push(message);
    if (this.queue.length > this.maxSize) {
      this.queue.shift(); // 超出限制，移除最旧的
    }
  }

  flush(ws: WebSocket) {
    while (this.queue.length > 0) {
      const message = this.queue.shift();
      ws.send(JSON.stringify(message));
    }
  }

  clear() {
    this.queue = [];
  }
}

// 使用
const messageQueue = new MessageQueue();

// 连接断开时缓存消息
if (ws.readyState !== WebSocket.OPEN) {
  messageQueue.enqueue(message);
} else {
  ws.send(JSON.stringify(message));
}

// 重连后发送缓存的消息
ws.onopen = () => {
  messageQueue.flush(ws);
};
```

---

### 5. 性能优化

**消息防抖：**

```typescript
// 对于频繁更新的事件（如房间容量变化），使用防抖
const debouncedRoomUpdate = debounce((roomInfo) => {
  updateRoomInfoUI(roomInfo);
}, 500);

ws.onRoomUpdate = (payload) => {
  debouncedRoomUpdate(payload.room_info);
};
```

**批量处理：**

```typescript
// 批量处理内容创建事件
const contentBuffer: any[] = [];
let flushTimeout: NodeJS.Timeout | null = null;

ws.onContentCreated = (payload) => {
  contentBuffer.push(payload);

  if (flushTimeout) {
    clearTimeout(flushTimeout);
  }

  flushTimeout = setTimeout(() => {
    // 一次性更新 UI
    updateFileListBatch(contentBuffer);
    contentBuffer.length = 0;
  }, 300);
};
```

---

## 完整示例

### 场景：实时文件共享房间

```typescript
import { ElizabethWebSocket } from "./websocket";
import { fetchRoomToken } from "./api";

class FileShareRoom {
  private ws: ElizabethWebSocket | null = null;
  private roomName: string;
  private fileList: any[] = [];
  private onlineUsers = new Set<string>();

  constructor(roomName: string, password?: string) {
    this.roomName = roomName;
    this.init(password);
  }

  private async init(password?: string) {
    try {
      // 1. 获取 Token
      const { token } = await fetchRoomToken(this.roomName, password);

      // 2. 建立 WebSocket 连接
      this.ws = new ElizabethWebSocket(this.roomName, token);
      this.setupEventHandlers();
      this.ws.connect();
    } catch (error) {
      console.error("初始化失败：", error);
      throw error;
    }
  }

  private setupEventHandlers() {
    if (!this.ws) return;

    // 连接成功
    this.ws.onConnected = (roomInfo) => {
      console.log("✅ 成功加入房间：", roomInfo.name);
      this.renderRoomInfo(roomInfo);
      this.loadInitialFiles();
    };

    // 文件上传
    this.ws.onContentCreated = (payload) => {
      if (payload.content_type === "file") {
        console.log("📁 新文件：", payload.file_name);
        this.fileList.push(payload);
        this.renderFileList();
        this.showNotification(`新文件：${payload.file_name}`);
      }
    };

    // 文件删除
    this.ws.onContentDeleted = (payload) => {
      console.log("🗑️ 文件已删除：", payload.file_name);
      this.fileList = this.fileList.filter((f) =>
        f.content_id !== payload.content_id
      );
      this.renderFileList();
    };

    // 用户加入
    this.ws.onUserJoined = (payload) => {
      console.log("👤 用户加入：", payload.user_id);
      this.onlineUsers.add(payload.user_id);
      this.renderOnlineUsers();
      this.showNotification("有新成员加入房间");
    };

    // 用户离开
    this.ws.onUserLeft = (payload) => {
      console.log("👋 用户离开：", payload.user_id);
      this.onlineUsers.delete(payload.user_id);
      this.renderOnlineUsers();
    };

    // 房间更新
    this.ws.onRoomUpdate = (payload) => {
      console.log("ℹ️ 房间信息更新：", payload.room_info);
      this.renderRoomInfo(payload.room_info);

      // 检查容量警告
      const { current_size, max_size } = payload.room_info;
      const usage = (current_size / max_size) * 100;
      if (usage > 90) {
        this.showWarning(`房间容量已使用 ${usage.toFixed(1)}%`);
      }
    };

    // 错误处理
    this.ws.onError = (error) => {
      console.error("❌ 错误：", error);
      this.showError(error);
    };

    // 连接丢失
    this.ws.onConnectionLost = () => {
      console.error("🔌 连接丢失");
      this.showError("与服务器断开连接，请刷新页面重试");
    };
  }

  private async loadInitialFiles() {
    // 通过 REST API 加载现有文件列表
    try {
      const response = await fetch(
        `/api/v1/rooms/${this.roomName}/contents?token=${this.ws?.token}`,
      );
      this.fileList = await response.json();
      this.renderFileList();
    } catch (error) {
      console.error("加载文件列表失败：", error);
    }
  }

  private renderRoomInfo(roomInfo: any) {
    const usagePercent = (roomInfo.current_size / roomInfo.max_size * 100)
      .toFixed(1);

    document.getElementById("room-name")!.textContent = roomInfo.name;
    document.getElementById("room-usage")!.textContent =
      `容量：${usagePercent}% (${formatBytes(roomInfo.current_size)} / ${
        formatBytes(roomInfo.max_size)
      })`;
    document.getElementById("room-visits")!.textContent =
      `访问：${roomInfo.current_times_entered} / ${roomInfo.max_times_entered}`;
  }

  private renderFileList() {
    const listEl = document.getElementById("file-list")!;
    listEl.innerHTML = this.fileList.map((file) => `
      <div class="file-item" data-id="${file.id}">
        <span class="file-name">${file.file_name}</span>
        <span class="file-size">${formatBytes(file.file_size)}</span>
        <button onclick="downloadFile(${file.id})">下载</button>
        <button onclick="deleteFile(${file.id})">删除</button>
      </div>
    `).join("");
  }

  private renderOnlineUsers() {
    document.getElementById("online-count")!.textContent =
      `在线：${this.onlineUsers.size}`;
  }

  private showNotification(message: string) {
    // 显示通知
    console.log("💬", message);
  }

  private showWarning(message: string) {
    // 显示警告
    console.warn("⚠️", message);
  }

  private showError(message: string) {
    // 显示错误
    console.error("❌", message);
  }

  disconnect() {
    this.ws?.close();
  }
}

// 工具函数
function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1048576) return (bytes / 1024).toFixed(2) + " KB";
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(2) + " MB";
  return (bytes / 1073741824).toFixed(2) + " GB";
}

// 使用
const room = new FileShareRoom("my-project", "password123");

// 页面卸载时断开连接
window.addEventListener("beforeunload", () => {
  room.disconnect();
});
```

---

## 调试技巧

### 1. 浏览器开发者工具

Chrome/Firefox DevTools → Network → WS 标签页可以查看：

- WebSocket 连接状态
- 所有发送/接收的消息
- 消息时间戳
- 连接断开原因

### 2. 日志记录

```typescript
class WebSocketLogger {
  static log(direction: 'send' | 'receive', message: any) {
    const timestamp = new Date().toISOString();
    const arrow = direction === 'send' ? '→' : '←';
    console.log(`[${timestamp}] ${arrow}`, message);
  }
}

// 在 sendMessage 中添加日志
private sendMessage(message: any) {
  WebSocketLogger.log('send', message);
  this.ws.send(JSON.stringify(message));
}

// 在 onmessage 中添加日志
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  WebSocketLogger.log('receive', message);
  this.handleMessage(message);
};
```

### 3. 连接状态监控

```typescript
const connectionStates = {
  [WebSocket.CONNECTING]: "CONNECTING",
  [WebSocket.OPEN]: "OPEN",
  [WebSocket.CLOSING]: "CLOSING",
  [WebSocket.CLOSED]: "CLOSED",
};

setInterval(() => {
  if (ws) {
    console.log("WebSocket 状态：", connectionStates[ws.readyState]);
  }
}, 5000);
```

---

## 常见问题

### Q1: 为什么连接总是断开？

**可能原因：**

- Token 已过期 → 使用 refresh_token 获取新 Token
- 未发送心跳 → 实现 PING/PONG 机制
- 网络不稳定 → 实现自动重连

### Q2: 如何知道消息是否发送成功？

WebSocket 是可靠传输（基于 TCP），消息会按顺序到达。如果需要确认：

- 设计应答机制（request-response 模式）
- 服务端收到后广播确认事件

### Q3: 多个标签页打开同一房间会怎样？

每个标签页会建立独立的 WebSocket 连接，都会接收到房间事件。需要注意：

- 每次连接都会增加进入次数
- 可使用 SharedWorker 或 BroadcastChannel 共享连接

### Q4: 消息顺序会乱吗？

不会。WebSocket 基于 TCP，保证消息顺序。同一连接的消息会按发送顺序到达。

---

**文档版本：** 1.0.0 **最后更新：** 2026-01-20 **API 版本：** v1
