# Elizabeth API 使用指南

本文档提供 Elizabeth 后端 REST API
的完整使用指南，包括详细的请求/响应示例和常见使用场景。

## 目录

- [基础信息](#基础信息)
- [房间管理 API](#房间管理-api)
- [Token 管理 API](#token-管理-api)
- [内容管理 API](#内容管理-api)
- [认证 API](#认证-api)
- [管理员 API](#管理员-api)
- [错误处理](#错误处理)
- [完整使用流程](#完整使用流程)

## 基础信息

### 服务地址

```
Docker Compose（推荐）: http://localhost:4001
后端直连（开发/调试）: http://127.0.0.1:4092
生产环境: https://your-domain.com
```

### API 版本

当前版本：`v1`

所有 API 路径均以 `/api/v1` 开头。

### 认证方式

Elizabeth 使用 JWT (JSON Web Token) 进行认证。大部分 API
需要在请求中携带有效的房间 Token。

**Token 传递方式：**

1. **Query 参数** (推荐用于 GET 请求):
   ```
   GET /api/v1/rooms/{name}/contents?token={your_token}
   ```

2. **Authorization Header**:
   ```
   Authorization: Bearer {your_token}
   ```

### 内容类型

所有请求和响应使用 `application/json` 格式。

---

## 房间管理 API

### 1. 创建房间

创建一个新的文件共享房间。

**端点：** `POST /api/v1/rooms/{name}`

**路径参数：**

- `name` (string, 必需): 房间名称，支持字母、数字、下划线、中文等

**查询参数：**

- `password` (string, 可选): 房间密码，设置后进入房间需要提供密码

**请求示例：**

```bash
# 创建无密码房间
curl -X POST "http://localhost:4001/api/v1/rooms/my-room"

# 创建带密码的房间
curl -X POST "http://localhost:4001/api/v1/rooms/secure-room?password=mypassword123"
```

**响应示例 (200 OK):**

```json
{
  "id": 1,
  "name": "my-room",
  "slug": "my-room-a1b2c3",
  "password": null,
  "status": 0,
  "max_size": 10737418240,
  "current_size": 0,
  "max_times_entered": 9223372036854775807,
  "current_times_entered": 0,
  "expire_at": "2026-01-27T10:30:00",
  "created_at": "2026-01-20T10:30:00",
  "updated_at": "2026-01-20T10:30:00",
  "permission": {
    "bits": 15
  }
}
```

**字段说明：**

- `id`: 房间唯一 ID
- `slug`: 房间唯一标识符（带随机后缀）
- `status`: 房间状态 (0=正常，1=关闭)
- `max_size`: 房间最大容量（字节）
- `current_size`: 当前已使用容量
- `max_times_entered`: 最大进入次数
- `current_times_entered`: 当前已进入次数
- `expire_at`: 房间过期时间
- `permission.bits`: 权限位掩码 (1=查看，2=编辑，4=删除，8=分享)

**错误响应：**

```json
// 400 - 房间已存在
{
  "error": "Room already exists",
  "status": 409
}

// 400 - 参数错误
{
  "error": "Invalid room name",
  "status": 400
}
```

---

### 2. 查找房间

根据房间名称或 slug 查找房间信息。

**端点：** `GET /api/v1/rooms/{name}`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**请求示例：**

```bash
curl -X GET "http://localhost:4001/api/v1/rooms/my-room"
```

**响应示例 (200 OK):**

```json
{
  "id": 1,
  "name": "my-room",
  "slug": "my-room-a1b2c3",
  "password": null,
  "status": 0,
  "max_size": 10737418240,
  "current_size": 0,
  "max_times_entered": 9223372036854775807,
  "current_times_entered": 0,
  "expire_at": "2026-01-27T10:30:00",
  "created_at": "2026-01-20T10:30:00",
  "updated_at": "2026-01-20T10:30:00",
  "permission": {
    "bits": 15
  }
}
```

**错误响应：**

```json
// 404 - 房间不存在
{
  "error": "Room not found: my-room",
  "status": 404
}
```

---

### 3. 删除房间

删除指定房间及其所有内容。需要房间 Token 且具有删除权限。

**端点：** `DELETE /api/v1/rooms/{name}`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token

**请求示例：**

```bash
curl -X DELETE "http://localhost:4001/api/v1/rooms/my-room?token=eyJhbGc..."
```

**响应示例 (200 OK):**

```json
{
  "message": "Room deleted successfully"
}
```

**错误响应：**

```json
// 401 - Token 无效
{
  "error": "Token is invalid or expired",
  "status": 401
}

// 403 - 权限不足
{
  "error": "Permission denied",
  "status": 403
}

// 404 - 房间不存在
{
  "error": "Room not found: my-room",
  "status": 404
}
```

---

### 4. 更新房间权限

更新房间的默认权限设置。

**端点：** `POST /api/v1/rooms/{name}/permissions`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token

**请求体：**

```json
{
  "permission": {
    "bits": 7
  }
}
```

**权限位说明：**

- `1` (0b0001): View - 查看权限
- `2` (0b0010): Edit - 编辑权限
- `4` (0b0100): Delete - 删除权限
- `8` (0b1000): Share - 分享权限
- `15` (0b1111): All - 所有权限

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/rooms/my-room/permissions?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{"permission": {"bits": 7}}'
```

**响应示例 (200 OK):**

```json
{
  "id": 1,
  "name": "my-room",
  "slug": "my-room-a1b2c3",
  "permission": {
    "bits": 7
  },
  ...
}
```

---

### 5. 更新房间设置

更新房间的容量限制、进入次数限制和过期时间。

**端点：** `POST /api/v1/rooms/{name}/settings`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token

**请求体：**

```json
{
  "max_size": 5368709120,
  "max_times_entered": 100,
  "expire_at": "2026-02-01T00:00:00"
}
```

**字段说明：**

- `max_size` (可选): 最大容量（字节），如 5GB = 5368709120
- `max_times_entered` (可选): 最大进入次数
- `expire_at` (可选): 过期时间 (ISO 8601 格式)

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/rooms/my-room/settings?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "max_size": 5368709120,
    "max_times_entered": 100,
    "expire_at": "2026-02-01T00:00:00"
  }'
```

**响应示例 (200 OK):**

```json
{
  "id": 1,
  "name": "my-room",
  "max_size": 5368709120,
  "max_times_entered": 100,
  "expire_at": "2026-02-01T00:00:00",
  ...
}
```

---

## Token 管理 API

### 1. 签发房间 Token

获取房间访问凭证。首次进入需要提供密码（如有），后续可使用已有 Token 刷新。

**端点：** `POST /api/v1/rooms/{name}/tokens`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**请求体：**

```json
{
  "password": "mypassword123",
  "token": "eyJhbGc...",
  "with_refresh_token": true
}
```

**字段说明：**

- `password` (可选): 房间密码，首次进入时需要（如果房间设置了密码）
- `token` (可选): 已有的有效 Token，用于刷新（不会增加进入次数）
- `with_refresh_token` (可选): 是否同时签发刷新 Token，默认 false

**请求示例：**

```bash
# 首次进入（需要密码）
curl -X POST "http://localhost:4001/api/v1/rooms/secure-room/tokens" \
  -H "Content-Type: application/json" \
  -d '{
    "password": "mypassword123",
    "with_refresh_token": true
  }'

# 刷新 Token（不增加进入次数）
curl -X POST "http://localhost:4001/api/v1/rooms/my-room/tokens" \
  -H "Content-Type: application/json" \
  -d '{
    "token": "eyJhbGc...",
    "with_refresh_token": true
  }'
```

**响应示例 (200 OK):**

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2026-01-20T11:30:00",
  "claims": {
    "jti": "550e8400-e29b-41d4-a716-446655440000",
    "room_id": 1,
    "room_name": "my-room",
    "iat": 1737368400,
    "exp": 1737372000,
    "permission": {
      "bits": 15
    }
  },
  "refresh_token": "ref_1234567890abcdef...",
  "refresh_token_expires_at": "2026-01-27T10:30:00"
}
```

**字段说明：**

- `token`: JWT 访问令牌（有效期 1 小时）
- `expires_at`: Token 过期时间
- `claims`: Token 载荷信息
  - `jti`: Token 唯一 ID
  - `room_id`: 房间 ID
  - `room_name`: 房间名称
  - `iat`: 签发时间戳
  - `exp`: 过期时间戳
  - `permission`: Token 权限
- `refresh_token`: 刷新令牌（有效期 7 天）
- `refresh_token_expires_at`: 刷新令牌过期时间

**错误响应：**

```json
// 403 - 密码错误
{
  "error": "Invalid room password",
  "status": 403
}

// 403 - 房间无法进入
{
  "error": "Room cannot be entered",
  "status": 403
}

// 404 - 房间不存在
{
  "error": "Room not found: my-room",
  "status": 404
}
```

---

### 2. 验证 Token

验证 Token 是否有效且未被撤销。

**端点：** `POST /api/v1/rooms/{name}/tokens/validate`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**请求体：**

```json
{
  "token": "eyJhbGc..."
}
```

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/rooms/my-room/tokens/validate" \
  -H "Content-Type: application/json" \
  -d '{"token": "eyJhbGc..."}'
```

**响应示例 (200 OK):**

```json
{
  "claims": {
    "jti": "550e8400-e29b-41d4-a716-446655440000",
    "room_id": 1,
    "room_name": "my-room",
    "iat": 1737368400,
    "exp": 1737372000,
    "permission": {
      "bits": 15
    }
  }
}
```

**错误响应：**

```json
// 401 - Token 无效
{
  "error": "Token is invalid or expired",
  "status": 401
}

// 401 - Token 已撤销
{
  "error": "Token revoked or not found",
  "status": 401
}
```

---

### 3. 撤销 Token

撤销指定的房间 Token。

**端点：** `POST /api/v1/rooms/{name}/tokens/revoke`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 当前有效的房间 Token

**请求体：**

```json
{
  "jti": "550e8400-e29b-41d4-a716-446655440000"
}
```

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/rooms/my-room/tokens/revoke?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{"jti": "550e8400-e29b-41d4-a716-446655440000"}'
```

**响应示例 (200 OK):**

```json
{
  "message": "Token revoked successfully"
}
```

---

### 4. 列出房间所有 Token

获取房间的所有活跃 Token 列表。

**端点：** `GET /api/v1/rooms/{name}/tokens`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token

**请求示例：**

```bash
curl -X GET "http://localhost:4001/api/v1/rooms/my-room/tokens?token=eyJhbGc..."
```

**响应示例 (200 OK):**

```json
[
  {
    "jti": "550e8400-e29b-41d4-a716-446655440000",
    "room_id": 1,
    "created_at": "2026-01-20T10:30:00",
    "expires_at": "2026-01-20T11:30:00",
    "status": 0,
    "permission": {
      "bits": 15
    }
  },
  {
    "jti": "660f9500-f39c-52e5-b827-557766551111",
    "room_id": 1,
    "created_at": "2026-01-20T10:35:00",
    "expires_at": "2026-01-20T11:35:00",
    "status": 0,
    "permission": {
      "bits": 7
    }
  }
]
```

---

## 内容管理 API

### 1. 列出房间内容

获取房间中所有文件和消息列表。

**端点：** `GET /api/v1/rooms/{name}/contents`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token

**请求示例：**

```bash
curl -X GET "http://localhost:4001/api/v1/rooms/my-room/contents?token=eyJhbGc..."
```

**响应示例 (200 OK):**

```json
[
  {
    "id": 1,
    "room_id": 1,
    "content_type": "file",
    "text": null,
    "file_name": "document.pdf",
    "file_size": 2048576,
    "file_path": "/uploads/rooms/1/document.pdf",
    "created_at": "2026-01-20T10:40:00",
    "updated_at": "2026-01-20T10:40:00"
  },
  {
    "id": 2,
    "room_id": 1,
    "content_type": "message",
    "text": "欢迎来到房间！",
    "file_name": null,
    "file_size": null,
    "file_path": null,
    "created_at": "2026-01-20T10:45:00",
    "updated_at": "2026-01-20T10:45:00"
  }
]
```

**字段说明：**

- `content_type`: 内容类型 ("file" 或 "message")
- `text`: 消息文本（仅消息类型）
- `file_name`: 文件名（仅文件类型）
- `file_size`: 文件大小（字节，仅文件类型）
- `file_path`: 文件存储路径（仅文件类型）

---

### 2. 准备上传

在实际上传文件前预留空间并获取上传许可。

**端点：** `POST /api/v1/rooms/{name}/contents/prepare`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token

**请求体：**

```json
{
  "files": [
    {
      "file_name": "document.pdf",
      "file_size": 2048576,
      "content_type": "application/pdf"
    },
    {
      "file_name": "image.png",
      "file_size": 1024000,
      "content_type": "image/png"
    }
  ]
}
```

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/rooms/my-room/contents/prepare?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "files": [
      {
        "file_name": "document.pdf",
        "file_size": 2048576,
        "content_type": "application/pdf"
      }
    ]
  }'
```

**响应示例 (200 OK):**

```json
{
  "reservation_id": 123,
  "reserved_size": 2048576,
  "room_remaining_size": 10735370164
}
```

**字段说明：**

- `reservation_id`: 预留 ID，上传时需要提供
- `reserved_size`: 本次预留的总大小
- `room_remaining_size`: 房间剩余可用空间

**错误响应：**

```json
// 413 - 超出容量限制
{
  "error": "Exceeds room capacity limit",
  "status": 413
}

// 403 - 无上传权限
{
  "error": "Permission denied",
  "status": 403
}
```

---

### 3. 上传文件

上传文件到房间。

**端点：** `POST /api/v1/rooms/{name}/contents`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token
- `reservation_id` (integer, 必需): 准备上传时获取的预留 ID

**请求体：** `multipart/form-data`

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/rooms/my-room/contents?token=eyJhbGc...&reservation_id=123" \
  -F "files=@/path/to/document.pdf" \
  -F "files=@/path/to/image.png"
```

**响应示例 (200 OK):**

```json
{
  "uploaded_files": [
    {
      "id": 1,
      "file_name": "document.pdf",
      "file_size": 2048576,
      "content_type": "application/pdf",
      "created_at": "2026-01-20T10:50:00"
    }
  ],
  "total_uploaded": 1,
  "room_current_size": 2048576
}
```

**错误响应：**

```json
// 400 - 预留 ID 不存在
{
  "error": "Reservation not found",
  "status": 400
}

// 413 - 文件大小超出预留
{
  "error": "File size exceeds reservation",
  "status": 413
}
```

---

### 4. 下载文件

下载房间中的文件。

**端点：** `GET /api/v1/rooms/{name}/contents/{content_id}/download`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug
- `content_id` (integer, 必需): 内容 ID

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token

**请求示例：**

```bash
curl -X GET "http://localhost:4001/api/v1/rooms/my-room/contents/1/download?token=eyJhbGc..." \
  -O -J
```

**响应：** 文件流（二进制数据）

**响应头：**

```
Content-Type: application/octet-stream
Content-Disposition: attachment; filename="document.pdf"
Content-Length: 2048576
```

---

### 5. 删除内容

删除房间中的文件或消息。

**端点：** `DELETE /api/v1/rooms/{name}/contents`

**路径参数：**

- `name` (string, 必需): 房间名称或 slug

**查询参数：**

- `token` (string, 必需): 有效的房间访问 Token

**请求体：**

```json
{
  "content_ids": [1, 2, 3]
}
```

**请求示例：**

```bash
curl -X DELETE "http://localhost:4001/api/v1/rooms/my-room/contents?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{"content_ids": [1, 2]}'
```

**响应示例 (200 OK):**

```json
{
  "deleted_count": 2,
  "freed_size": 3072576,
  "room_current_size": 0
}
```

**错误响应：**

```json
// 403 - 无删除权限
{
  "error": "Permission denied",
  "status": 403
}

// 404 - 内容不存在
{
  "error": "Content not found",
  "status": 404
}
```

---

## 认证 API

### 1. 刷新访问令牌

使用刷新令牌获取新的访问令牌和刷新令牌对。

**端点：** `POST /api/v1/auth/refresh`

**请求体：**

```json
{
  "refresh_token": "ref_1234567890abcdef..."
}
```

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/auth/refresh" \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "ref_1234567890abcdef..."}'
```

**响应示例 (200 OK):**

```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "ref_0987654321fedcba...",
  "expires_at": "2026-01-20T11:30:00",
  "refresh_token_expires_at": "2026-01-27T10:30:00"
}
```

**错误响应：**

```json
// 401 - 刷新令牌无效
{
  "error": "Invalid refresh token",
  "status": 401
}

// 401 - 刷新令牌已过期
{
  "error": "Refresh token expired",
  "status": 401
}
```

---

### 2. 撤销刷新令牌

撤销指定的刷新令牌。

**端点：** `POST /api/v1/auth/revoke`

**请求体：**

```json
{
  "refresh_token": "ref_1234567890abcdef..."
}
```

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/auth/revoke" \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "ref_1234567890abcdef..."}'
```

**响应示例 (200 OK):**

```json
{
  "message": "Refresh token revoked successfully"
}
```

---

### 3. 清理过期令牌

清理数据库中所有过期的刷新令牌（管理员操作）。

**端点：** `POST /api/v1/auth/cleanup`

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/auth/cleanup"
```

**响应示例 (200 OK):**

```json
{
  "deleted_count": 42
}
```

---

## 管理员 API

### 1. 列出所有房间

列出所有未过期的房间（包括无限制房间）。

**端点：** `GET /api/v1/admin/rooms`

**请求示例：**

```bash
curl -X GET "http://localhost:4001/api/v1/admin/rooms"
```

**响应示例 (200 OK):**

```json
[
  {
    "id": 1,
    "name": "room-1",
    "slug": "room-1-a1b2c3",
    "current_size": 2048576,
    "max_size": 10737418240,
    "current_times_entered": 5,
    "max_times_entered": 9223372036854775807,
    "expire_at": "2026-01-27T10:30:00",
    "created_at": "2026-01-20T10:30:00"
  },
  ...
]
```

---

### 2. 运行房间垃圾回收

手动触发房间垃圾回收，清理过期房间及其内容。

**端点：** `POST /api/v1/admin/gc`

**请求示例：**

```bash
curl -X POST "http://localhost:4001/api/v1/admin/gc"
```

**响应示例 (200 OK):**

```json
{
  "cleaned_rooms": 3,
  "freed_size": 15728640,
  "execution_time_ms": 245
}
```

---

## 错误处理

### HTTP 状态码

| 状态码 | 说明                         |
| ------ | ---------------------------- |
| 200    | 请求成功                     |
| 400    | 请求参数错误                 |
| 401    | 未授权（Token 无效或已过期） |
| 403    | 权限不足                     |
| 404    | 资源不存在                   |
| 409    | 资源冲突（如房间已存在）     |
| 413    | 请求实体过大（超出容量限制） |
| 500    | 服务器内部错误               |

### 错误响应格式

所有错误响应遵循统一格式：

```json
{
  "error": "错误描述信息",
  "status": 400,
  "details": {
    "field": "具体错误字段",
    "message": "详细错误信息"
  }
}
```

### 常见错误

**Token 相关：**

```json
{
  "error": "Token is invalid or expired",
  "status": 401
}
```

**权限相关：**

```json
{
  "error": "Permission denied",
  "status": 403
}
```

**资源不存在：**

```json
{
  "error": "Room not found: my-room",
  "status": 404
}
```

**容量限制：**

```json
{
  "error": "Exceeds room capacity limit",
  "status": 413
}
```

---

## 完整使用流程

### 场景 1: 创建房间并上传文件

```bash
# 1. 创建房间
curl -X POST "http://localhost:4001/api/v1/rooms/my-project?password=secret123"

# 响应:
# {
#   "id": 1,
#   "name": "my-project",
#   "slug": "my-project-a1b2c3",
#   ...
# }

# 2. 获取访问 Token
curl -X POST "http://localhost:4001/api/v1/rooms/my-project/tokens" \
  -H "Content-Type: application/json" \
  -d '{
    "password": "secret123",
    "with_refresh_token": true
  }'

# 响应:
# {
#   "token": "eyJhbGc...",
#   "expires_at": "2026-01-20T11:30:00",
#   "refresh_token": "ref_...",
#   ...
# }

# 3. 准备上传文件
curl -X POST "http://localhost:4001/api/v1/rooms/my-project/contents/prepare?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "files": [
      {
        "file_name": "report.pdf",
        "file_size": 5242880,
        "content_type": "application/pdf"
      }
    ]
  }'

# 响应:
# {
#   "reservation_id": 101,
#   "reserved_size": 5242880,
#   ...
# }

# 4. 上传文件
curl -X POST "http://localhost:4001/api/v1/rooms/my-project/contents?token=eyJhbGc...&reservation_id=101" \
  -F "files=@/path/to/report.pdf"

# 响应:
# {
#   "uploaded_files": [
#     {
#       "id": 1,
#       "file_name": "report.pdf",
#       "file_size": 5242880,
#       ...
#     }
#   ],
#   ...
# }
```

---

### 场景 2: 加入现有房间并下载文件

```bash
# 1. 获取房间信息
curl -X GET "http://localhost:4001/api/v1/rooms/my-project"

# 2. 获取访问 Token（需要密码）
curl -X POST "http://localhost:4001/api/v1/rooms/my-project/tokens" \
  -H "Content-Type: application/json" \
  -d '{
    "password": "secret123",
    "with_refresh_token": true
  }'

# 3. 列出房间内容
curl -X GET "http://localhost:4001/api/v1/rooms/my-project/contents?token=eyJhbGc..."

# 响应:
# [
#   {
#     "id": 1,
#     "content_type": "file",
#     "file_name": "report.pdf",
#     "file_size": 5242880,
#     ...
#   }
# ]

# 4. 下载文件
curl -X GET "http://localhost:4001/api/v1/rooms/my-project/contents/1/download?token=eyJhbGc..." \
  -O -J
```

---

### 场景 3: Token 刷新流程

```bash
# 1. Token 即将过期时，使用刷新令牌获取新 Token
curl -X POST "http://localhost:4001/api/v1/auth/refresh" \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "ref_1234567890abcdef..."
  }'

# 响应:
# {
#   "access_token": "eyJhbGc...",  # 新的访问令牌
#   "refresh_token": "ref_...",     # 新的刷新令牌
#   "expires_at": "2026-01-20T12:30:00",
#   ...
# }

# 2. 使用新的访问令牌继续操作
curl -X GET "http://localhost:4001/api/v1/rooms/my-project/contents?token=新的eyJhbGc..."
```

---

### 场景 4: 管理房间权限

```bash
# 1. 创建房间（拥有者获得完整权限）
curl -X POST "http://localhost:4001/api/v1/rooms/team-space"

# 2. 获取 Token
curl -X POST "http://localhost:4001/api/v1/rooms/team-space/tokens" \
  -H "Content-Type: application/json" \
  -d '{"with_refresh_token": true}'

# 3. 更新房间权限（只允许查看和编辑，禁止删除）
curl -X POST "http://localhost:4001/api/v1/rooms/team-space/permissions?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "permission": {
      "bits": 3
    }
  }'
# bits = 3 = 0b0011 = View(1) + Edit(2)

# 4. 更新房间设置（限制容量和过期时间）
curl -X POST "http://localhost:4001/api/v1/rooms/team-space/settings?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "max_size": 1073741824,
    "max_times_entered": 50,
    "expire_at": "2026-02-01T00:00:00"
  }'
```

---

### 场景 5: 批量删除文件

```bash
# 1. 列出房间内容
curl -X GET "http://localhost:4001/api/v1/rooms/my-room/contents?token=eyJhbGc..."

# 响应:
# [
#   {"id": 1, "file_name": "old-file-1.pdf", ...},
#   {"id": 2, "file_name": "old-file-2.png", ...},
#   {"id": 3, "file_name": "keep-this.docx", ...}
# ]

# 2. 删除指定文件（ID 1 和 2）
curl -X DELETE "http://localhost:4001/api/v1/rooms/my-room/contents?token=eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "content_ids": [1, 2]
  }'

# 响应:
# {
#   "deleted_count": 2,
#   "freed_size": 3145728,
#   "room_current_size": 1048576
# }
```

---

## 最佳实践

### 1. Token 管理

- **存储安全**: 将 Token 存储在 localStorage 或 sessionStorage，不要暴露在 URL
  中
- **自动刷新**: 在 Token 过期前使用 refresh_token 自动刷新
- **权限最小化**: 根据实际需求设置最小必要权限

### 2. 文件上传

- **分步上传**: 先调用 `prepare` 预留空间，再执行实际上传
- **错误处理**: 捕获 413 错误，提示用户容量不足
- **进度显示**: 使用 XMLHttpRequest 或 Fetch API 显示上传进度

### 3. 错误处理

- **统一处理**: 实现全局错误拦截器处理 401/403 错误
- **友好提示**: 将错误信息转换为用户友好的提示
- **重试机制**: 对 5xx 错误实施指数退避重试

### 4. 性能优化

- **并发控制**: 限制同时上传的文件数量
- **分块上传**: 大文件使用分块上传（未来支持）
- **缓存策略**: 合理使用 HTTP 缓存头

---

## 附录

### A. 权限位计算

权限使用位掩码表示，可以通过位运算组合：

```javascript
const Permission = {
  VIEW: 1, // 0b0001
  EDIT: 2, // 0b0010
  DELETE: 4, // 0b0100
  SHARE: 8, // 0b1000
};

// 组合权限
const viewAndEdit = Permission.VIEW | Permission.EDIT; // 3
const all = Permission.VIEW | Permission.EDIT | Permission.DELETE |
  Permission.SHARE; // 15

// 检查权限
const hasEditPermission = (bits & Permission.EDIT) !== 0;
```

### B. 时间格式

所有时间字段使用 ISO 8601 格式：

```
2026-01-20T10:30:00    # 本地时间
2026-01-20T10:30:00Z   # UTC 时间
```

### C. 文件大小单位

```
1 KB = 1,024 bytes
1 MB = 1,048,576 bytes
1 GB = 1,073,741,824 bytes
10 GB = 10,737,418,240 bytes (默认房间容量)
```

---

**文档版本：** 1.0.0 **最后更新：** 2026-01-20 **API 版本：** v1
