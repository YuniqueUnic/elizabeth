# Elizabeth 架构与授权开发指引

本文档是当前实现的架构真相。授权相关代码、数据库迁移和前端类型必须与本文档保持一致；旧的 permission bitmask、`rooms.permission` 和 `permissions` API 已删除。

## 1. 分层

```text
HTTP/WebSocket adapters
  -> handlers / services
    -> authorization guard + domain rules
      -> repositories / sqlx / storage
```

- `board-protocol`：跨边界的 DTO、Room 模型、`Capability`、`Scope`、`Grant`。不依赖 HTTP 或数据库连接。
- `board::authz`：纯授权引擎和角色表缓存。`authorize()` 只接收已组装的主体、资源和授权集，不执行 I/O。
- handlers：编排 Room Gate、授权和业务命令。每个需要房间身份的端点都先验证身份码，再调用 `Authz::require()`。
- repositories：唯一的持久化入口。角色变更必须在同一事务中递增 `rooms.roles_version`。
- web：只消费生成的 TypeScript 契约；服务端始终是最终授权者。

## 2. 统一身份码

房间访问 JWT 是唯一的房间身份码。它包含 `room_id`、`room_name`、`jti`、过期时间和 `role`；role 只是角色表索引，不是授权快照。

以下三种输入在房间端点上具有同一语义：

```http
Authorization: Bearer <room-jwt>
X-API-Key: <room-jwt>
GET /api/v1/rooms/demo/contents?token=<room-jwt>
```

`X-API-Key` 只是机器调用的便捷传输头，不产生第二种凭证、数据库表或权限模型。查询参数仅为浏览器媒体链接保留，普通 API 客户端应使用 Bearer 或 X-API-Key。

部署级 `X-Elizabeth-Admin-Token` 不与房间身份码合并。它没有房间角色，只用于运维端点，并可在签发房间 admin 身份码时作为 bootstrap credential。admin secret 永远不能写入日志、JWT 或前端持久化存储。

前端输入身份码后的流程：

1. 将身份码提交到 `POST /rooms/{name}/tokens/validate` 或作为 `token` 请求签发参数。
2. 从响应 JWT claims 读取当前 role 和 jti；从签发响应读取 capabilities 作为初始 UI 快照。
3. 角色能力变化通过 `roles_changed` 房间事件触发失效和重新查询；UI 隐藏按钮不能替代服务端校验。

## 3. 授权模型

授权决策是 `role × capability × resource`：

```rust
authorize(grants, principal, capability, resource) -> Allow | Deny(reason)
```

- `Principal`：`jti`、`room_id`、`role`。
- `Resource::Room`：房间级设置、角色、分享、删除等操作。
- `Resource::Content`：带 `room_id`、内容类型和 `created_by_jti` 的消息或文件。
- `Scope::Any`：允许房间内任意资源。
- `Scope::Own`：仅当 `created_by_jti == principal.jti` 时允许；历史 NULL owner 永不满足 Own。
- room 不一致、角色不存在、能力不存在、Own 不匹配都 fail-closed。

当前 capability 契约：

`room.share`, `room.settings.update`, `room.roles.manage`, `room.delete`, `msg.read`, `msg.send`, `msg.copy`, `msg.edit`, `msg.delete`, `file.list`, `file.preview`, `file.download`, `file.upload`, `file.delete`, `file.policy.manage`。

系统角色在每个房间独立种子化：

- `admin`：全部能力 Any。
- `editor`：消息读写复制编辑；消息/文件删除 Own；文件列表、预览、下载、上传。
- `reader`：消息读取复制；文件列表、预览、下载。

角色修改立即影响所有会话。缓存只按 `room_id` 存储，并使用 `roles_version` 失效；JWT 中的 role 不应被当作能力缓存。

## 4. 数据与迁移

`20260902100000_room_roles_authz.sql` 同时维护 SQLite 和 PostgreSQL：

- `room_roles(room_id, role_key, display_name, capabilities, is_system)`。
- `rooms.default_role_key` 和 `rooms.roles_version`。
- `room_tokens.role_key`。
- `room_contents.created_by_jti`。
- 删除 `rooms.permission`，并重建依赖视图。

修改 schema 时：

1. 在 `crates/board/migrations/` 与 `migrations_pg/` 写增量迁移，保留历史数据。
2. 使用现有 sqlx 启动迁移流程；SQLite 迁移前由 `VACUUM INTO` 生成备份。
3. 不写运行时 `IF NOT EXISTS` 兼容胶水，不删库重建。
4. 迁移后验证角色种子、旧 token role_key、NULL owner 的 fail-closed 行为。

## 5. 新增端点的强制流程

1. 识别资源所属 `room_id`。
2. 使用 `AuthToken` 提取身份码并调用 `verify_room_token`，完成签名、jti、房间状态和 Room Gate 检查。
3. 构造 `Authz::for_claims`，只用 `authz.require(capability, resource)` 做授权。
4. 对 Own 操作从数据库读取 `created_by_jti`，禁止使用请求体提供的 owner。
5. 业务写入和关联版本更新放在 repository 事务内。
6. 角色矩阵改变后广播 `RoomUpdateReason::RolesChanged`。
7. 在 `crates/board/src/tests/` 或 `crates/board/tests/` 增加拒绝和允许矩阵测试。

禁止：直接读取旧 permission 位、在 handler 中比较 role 字符串绕过引擎、将能力写入 JWT 作为长期真相、为旧端点增加兼容别名。

## 6. 质量门禁

```bash
cargo fmt --all
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p elizabeth-board --features typescript-export
(cd web && bun typecheck && bun run build)
```

生成类型的唯一入口是 `cargo build -p elizabeth-board --features typescript-export`。不要手工编辑 `web/types/generated/`。
