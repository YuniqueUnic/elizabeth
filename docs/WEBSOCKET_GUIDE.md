# Elizabeth WebSocket 指南（简版）

## 端点

WebSocket 路径固定为：`/api/v1/ws`。

- Docker Compose（推荐）：`ws://localhost:4001/api/v1/ws`
- 后端直连（开发/调试）：`ws://127.0.0.1:4092/api/v1/ws`
- 生产环境：`wss://<your-domain>/api/v1/ws`

> Docker Compose 默认由网关 `docker/gateway/nginx.conf` 代理该路径。

## 客户端连接（最小示例）

1. 先签发房间 token：

```bash
BASE=http://localhost:4001/api/v1
curl -X POST "$BASE/rooms/my-room/tokens" \
  -H "Content-Type: application/json" \
  -d '{"password":null,"with_refresh_token":false}'
```

2. 用 token 建立 WebSocket 连接（示意）：

```js
const ws = new WebSocket("ws://localhost:4001/api/v1/ws");
ws.onopen = () =>
  ws.send(JSON.stringify({
    message_type: "connect",
    payload: { room_name: "my-room", token: "<ROOM_TOKEN>" },
    timestamp: Date.now(),
  }));
```

## 详细版本

- 完整协议/事件表：`WEBSOCKET_GUIDE_FULL.md`
