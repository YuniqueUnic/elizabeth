# 房间上传与权限体系重构实录

## 背景与问题陈述

最近在排查文件房间服务时，我们发现三个长期存在的隐患：

1. **上传流程缺乏前置校验。** 现有实现是在服务器端完整接收文件后，再去判断
   `current_size + new_size` 是否超过上限。这不仅浪费
   IO，也在并发场景下存在竞争条件。
2. **分享权限语义缺失。** `RoomPermission::SHARE`
   只是一个摆设，所有房间都可以直接通过 `/{room_name}`
   访问，想做“私有链接”根本无法实现。
3. **访问 token 缺少生命周期管理。** 一把钥匙开所有门，刷新操作会反复签发新
   token，但旧 token 依旧有效，风险可想而知。

## 原始实现剖析

- `handlers/content.rs::upload_contents` 逐块写文件到磁盘，最后才进行 size
  check。一旦超限要回滚所有已写入的块，耗时且不可靠。
- `Room` 只有 `name` 字段，路由层也完全依赖这个名称。即便我们想构造
  `room_name_uuid` 的链接，数据库也无法区分“展示名称”与“访问 slug”。
- `issue_token` 允许带上旧 token“续签”，但不会吊销原来的
  token，等于给客户端送了一把万能钥匙。

## 优化方向与落地实现

### 1. 上传预检 & 空间预留

- 新增表 `room_upload_reservations`（见
  `006_create_room_upload_reservations_table.sql`），记录
  `room_id`、`token_jti`、`reserved_size`、`expires_at` 等信息。
- 新接口 `POST /api/v1/rooms/{slug}/contents/prepare`：前端汇总文件列表
  `{name,size}` 后一次性请求预留空间，服务端原子地校验并更新
  `rooms.current_size`，同时写入 reservation 记录。
- 预留记录 10 秒自动过期（spawn 定时释放），若客户端未在期限内上传会自动回滚。
- 实际上传时携带
  `reservation_id`，逐个校验文件名、大小与预案一致，全部写入成功后通过
  `consume_reservation` 固化占用；否则删除临时文件并释放差额。

这一结构保证了“先占坑再上传”，在并发上传下也能严格保证容量上限。

### 2. 分享权限落地：slug + 配置接口

- 在 `rooms` 表中新增 `slug` 字段（见 `007_add_room_slug.sql`），`Room`
  模型同步扩展。
- 默认情况下 `slug == name`，当关闭分享时自动生成 `name_uuid` 的私有
  slug，并确保唯一性。
- 新增 `POST /api/v1/rooms/{slug}/permissions`，首个拥有删除权限的用户可以配置
  `{edit, share, delete}` 组合：
  - 若打开分享，slug 会回落为用户设置的房间名（需保证无冲突）。
  - 若关闭分享，生成私有 slug，并禁止通过原始名称访问（`find` 接口对同名房间返回
    403 而不再偷偷创建新房间）。
- API 响应始终回传最新的 `slug`，前端据此分享或存储深链接。

### 3. token 生命周期管理

- `issue_token` 现在在签发新 token 成功后，立即调用
  `SqliteRoomTokenRepository::revoke` 将旧 token 标记为失效。
- 新增集成测试 `test_token_refresh_revokes_old_token`，验证刷新后旧 token
  无法继续通过验证接口。

## 其他工程细节

- Content handler 使用 slug 作为存储目录，避免房间名变更带来的混淆。
- 新的 SQL 查询通过 `cargo sqlx prepare` 生成缓存，配合 `SQLX_OFFLINE=true` 运行
  `cargo check/test`，确保 CI 可离线编译。
- 测试覆盖：原有单测全部绿灯，并补充了上传、分享、token 三条核心链路的集成测试。

## 结果与体会

这轮调整将房间上传与权限管理彻底拉回了“先规划再执行”的轨道：

- 上传容量在客户端即可给出明确反馈，服务器只做一次性写入，避免冗余 IO。
- 分享权限从“摆设”变为“契约”：slug 简洁明了，分享/私有切换一键完成。
- Token 刷新终于成为安全工具，而不是漏洞放大器。

下一步可以考虑把 `reservation`
的状态暴露给前端诊断（例如主动查询预留列表），并在 UI
上引导用户及时完成上传。总体而言，这次改造清晰地梳理了“访问-上传-分享”三条主线，让房间服务更贴近业务预期。
