# Elizabeth API 使用指南（简版）

建议优先看 OpenAPI UI：`/api/v1/scalar`（JSON：`/api/v1/openapi.json`）。

## Base URL

- Docker Compose（推荐）：`http://localhost:4001/api/v1`
- 后端直连（开发/调试）：`http://127.0.0.1:4092/api/v1`

下面示例用 `BASE=http://localhost:4001/api/v1`。

## 认证（Token）

大多数接口需要房间 token，支持两种传递方式：

- Query：`?token=...`（适合 GET）
- Header：`Authorization: Bearer ...`

## 最常用的几个接口

```bash
BASE=http://localhost:4001/api/v1

# 1) 创建房间（可选 password）
curl -X POST "$BASE/rooms/my-room"

# 2) 签发房间 token（如房间有密码则传 password）
curl -X POST "$BASE/rooms/my-room/tokens" \
  -H "Content-Type: application/json" \
  -d '{"password":null,"with_refresh_token":false}'

# 3) 列出内容（Query token）
curl "$BASE/rooms/my-room/contents?token=<ROOM_TOKEN>"
```

## 详细版本

- 详细接口与完整流程：`API_GUIDE_FULL.md`
