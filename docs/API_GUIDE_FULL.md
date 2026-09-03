# Elizabeth API 指南

本文档描述当前 role/capability API。所有房间 API 的身份输入都是同一个房间 JWT 身份码；服务端授权仍以房间数据库角色矩阵为准。

## 基础

默认地址为 `http://localhost:4092`，API 前缀为 `/api/v1`。JSON 请求使用 `Content-Type: application/json`。

房间身份码可通过以下任一方式传递：

```http
Authorization: Bearer <room-jwt>
X-API-Key: <room-jwt>
```

浏览器媒体链接还支持 `?token=<room-jwt>`；脚本和普通 API 请求不要把身份码放进 URL。

## 房间

### 创建房间

```http
POST /api/v1/rooms/{name}
{"password":"optional room password"}
```

响应 `RoomView` 包含 `id`、`name`、`slug`、容量、过期时间、`default_role_key` 和 `roles_version`，不包含密码或旧 permission 位。

### 获取房间

```http
GET /api/v1/rooms/{name}
```

不存在且名称合法时按部署默认配置创建；已存在的房间返回当前公开设置。

### 更新房间设置

```http
PUT /api/v1/rooms/{name}/settings
Authorization: Bearer <admin-jwt>

{
  "password": "new password",
  "remove_password": false,
  "expire_at": null,
  "max_times_entered": 100,
  "max_size": 52428800,
  "default_role_key": "reader"
}
```

需要 `room.settings.update`。默认角色必须存在于当前房间。

### 删除房间

```http
DELETE /api/v1/rooms/{name}
X-API-Key: <admin-jwt>
```

需要 `room.delete`。

## 身份码与角色

### 签发房间身份码

```http
POST /api/v1/rooms/{name}/tokens
{
  "password": "room password",
  "token": "existing room jwt",
  "role": "reader",
  "with_refresh_token": true
}
```

`password` 与 `token` 按进入/续签场景使用。省略 `role` 使用房间 `default_role_key`。请求非默认角色需要已有身份码的 `room.roles.manage`，或使用部署级 `X-Elizabeth-Admin-Token` bootstrap credential；密码房间首次 bootstrap admin 还必须提供房间密码。

响应：

```json
{
  "token": "<room-jwt>",
  "claims": {
    "jti": "...",
    "room_id": 1,
    "room_name": "demo",
    "role": "reader",
    "exp": 0
  },
  "expires_at": "2026-09-02T12:00:00",
  "capabilities": [
    {"capability":"msg.read","scope":"any"},
    {"capability":"msg.delete","scope":"own"}
  ],
  "refresh_token": "optional"
}
```

`capabilities` 是签发时的 UI 快照，不是长期授权真相。角色矩阵改变后必须重新获取或刷新身份码。

### 校验身份码

```http
POST /api/v1/rooms/{name}/tokens/validate
{"token":"<room-jwt>"}
```

返回 claims 中的 `role`、`jti` 和房间绑定信息。角色被删除、token 被撤销、房间状态不允许进入时拒绝。

### 刷新身份码

```http
POST /api/v1/auth/refresh
{"refresh_token":"<refresh-token>"}
```

刷新过程从数据库 token 记录读取 `role_key`，然后按当前房间角色矩阵生成新 JWT 和 capabilities。

### 列出、撤销身份码

```http
GET  /api/v1/rooms/{name}/tokens
POST /api/v1/rooms/{name}/tokens/revoke
```

两者都需要 `room.roles.manage`。撤销只改变会话状态，不删除角色定义。

## 角色矩阵

所有角色端点都需要 `room.roles.manage`，角色只在所属房间有效：

```http
GET    /api/v1/rooms/{name}/roles
POST   /api/v1/rooms/{name}/roles
PUT    /api/v1/rooms/{name}/roles/{role_key}
DELETE /api/v1/rooms/{name}/roles/{role_key}
```

创建/更新请求：

```json
{
  "role_key": "moderator",
  "display_name": "Moderator",
  "capabilities": [
    {"capability":"msg.read","scope":"any"},
    {"capability":"msg.delete","scope":"own"}
  ]
}
```

更新请求不包含 `role_key`。系统角色 `admin`、`editor`、`reader` 不可删除；默认加入角色也不可直接删除。删除响应的 `affected_tokens` 表示仍引用该角色的活动会话数，这些会话会立即失去能力。

可用 capability：

- 房间：`room.share`、`room.settings.update`、`room.roles.manage`、`room.delete`
- 消息：`msg.read`、`msg.send`、`msg.copy`、`msg.edit`、`msg.delete`
- 文件：`file.list`、`file.preview`、`file.download`、`file.upload`、`file.delete`、`file.policy.manage`

`scope` 只能是 `any` 或 `own`。`own` 只适用于消息编辑/删除和文件删除，并要求资源 `created_by_jti` 等于身份码 jti。

## 内容与文件

消息、URL、上传、下载、删除、下载策略端点均先做身份码验证，再按资源类型调用 capability guard。典型调用：

```http
GET  /api/v1/rooms/{name}/messages       # msg.read
POST /api/v1/rooms/{name}/messages       # msg.send
POST /api/v1/rooms/{name}/contents/prepare # file.upload
GET  /api/v1/rooms/{name}/contents/{id}/download # file.download
DELETE /api/v1/rooms/{name}/contents    # msg.delete 或 file.delete
```

批量删除中任一资源不满足 Own/Any 即整体拒绝。历史内容没有 owner 时不能满足 Own。

## 错误与安全

- `401`：身份码缺失、格式错误、签名错误、过期或已撤销。
- `403`：Room Gate 通过但缺少 capability，或资源不属于当前房间。
- `404`：房间/角色/资源不存在。
- `409`：角色 key 冲突、系统角色删除、默认角色删除。

不要把 room JWT、refresh token 或 admin credential 写日志、放入分析事件或长期 URL。`X-Elizabeth-Admin-Token` 是部署级运维凭证，不是房间角色，也不能替代房间身份码。

## 开发与迁移

新增授权端点必须遵循：`AuthToken` -> `verify_room_token` -> `Authz::for_claims` -> `authz.require` -> repository command。新增角色字段必须同时更新 SQLite/PostgreSQL sqlx migration、DTO、生成类型和测试；不添加旧 permission 兼容字段或路由。

验证命令：

```bash
cargo fmt --all
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p elizabeth-board --features typescript-export
(cd web && bun typecheck && bun run build)
```
