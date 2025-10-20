# 背景与问题陈述

在现有的 Elizabeth board 服务里，房间概念已经存在，但内容管理仍停留在 TODO
状态：缺少凭证体系、没有内容上传/下载接口，也无法控制磁盘配额。这导致房间一旦公开，任何人都可以任意操作；相反，如果房间加密，又无从让通过密码校验的访客继续访问内容。本轮迭代的目标，是为房间补齐一套完整的内容处理链：通过房间签发的
JWT
凭证关联权限，围绕该凭证实现内容的列举、上传、下载与删除，并在所有环节严格校验配额与状态。

# 原始实现及分析

原始代码只实现了房间的 CRUD，`handlers/content.rs` 里全是需求注释；Repository
层虽然建好了 `room_contents` 表，但没有列举/删除等接口，`Room`
也没有与凭证的联动。最关键的缺口是**房间凭证**：没有凭证，就无法在 API
层做身份与权限区分，也无法落地后续内容校验和配额限制。

进一步分析现状时，我梳理出几个核心制约因素：

1. **配置缺失**：服务配置里没有 `jwt_secret`，难以生成稳定的签名；
2. **状态共享不足**：Axum 路由层只拿到了数据库连接池，没有统一的应用状态；
3. **Repository 能力不足**：`room_content_repository` 只能
   `create/update/delete` 整个房间内容，无法按 ID 查询或批量删除；
4. **路由空洞**：`route::room::api_router`
   只注册了房间相关接口，得为凭证与内容扩展新的路由；
5. **测试空白**：既没有针对凭证的单元测试，也没有内容接口的集成测试，无法验证数据一致性。

# 优化方向与实现

## 1. 构建凭证体系

- 在 `configrs::AppConfig` 新增 `jwt_secret`，并在 CLI 合并逻辑中支持覆写；
- 引入 `AppState`，同时持有数据库连接池与 `RoomTokenService`，成为 Axum
  `State`；
- 新增 `RoomToken` 模型、`room_tokens` 表迁移及对应的 Repository，记录 token 的
  `jti`、过期时间与撤销状态；
- 编写 `RoomTokenService`，用 `jsonwebtoken` 生成/验证 HS256 JWT，claims
  中携带房间 ID、名称、权限以及到期时间；
- 在 `handlers/rooms.rs` 增加
  `issue_token/validate_token/list_tokens/revoke_token`
  四个接口，并在签发时校验房间密码或已有 token。

## 2. 内容处理链路

- 将 `Room::new` 默认权限调整为 `with_all()`，确保新房间拥有完整读写能力；
- 在 `handlers/content.rs` 完成
  `list_contents/upload_contents/delete_contents/download_content`，引入
  `sanitize-filename`、`mime_guess` 等工具，对上传文件进行清洗并落盘至
  `storage/rooms/{room}`；
- 上传时流式写入磁盘、累计文件大小，超过房间 `max_size` 即拒绝，并创建
  `room_contents` 记录后同步刷新房间 `current_size`；
- 删除时基于房间全部内容过滤目标 ID，先删除磁盘文件再调用 Repository
  批量删库，最后扣减房间容量；
- 下载时通过 `ReaderStream` 流式返回文件，设置 `Content-Disposition` 与正确的
  MIME。

## 3. 路由与共享校验

- 为统一的 token 校验封装
  `handlers::token::verify_room_token`，在房间与内容处理链之间复用；
- 调整 `route::room::api_router`，使用 `axum` 原生 `route`
  拼装所有房间、凭证与内容接口路径；
- Repository 新增 `list_by_room`、`delete_by_ids` 等能力，并使用
  `sqlx::QueryBuilder` 处理动态 IN 语句；
- 通过 `cargo sqlx prepare` 生成最新的 `.sqlx` 缓存，保证离线校验通过。

## 4. 自动化测试

- 扩展原有的 API 集成测试，新增
  `test_room_token_and_content_flow`，覆盖“创建房间 → 签发 token → 上传 → 列表 →
  下载 → 删除”的闭环；
- 在流程中断言 403/500 等异常时返回的错误消息，借助测试快速定位权限或 SQL
  映射问题；
- 调整测试工具函数以适配新的 `AppState`，避免状态不一致。

# 实施过程中的痛点与心得

1. **Axum + Utoipa 的路由冲突**：初次直接把所有 handler 丢给
   `utoipa_axum::routes!`，结果遇到 `Overlapping method route` 的
   panic。原因在于宏内部以 Axum v0.7 风格注册路径，无法与 v0.8
   默认的冒号语法共存。最终改为手写
   `route("/api/v1/rooms/{name}", post(create))` 方式解决。

2. **动态 SQLx 查询的列名映射**：使用 `QueryBuilder` 构建多 ID
   查询时，运行时提示 `no column found for name: content_type`。排查发现
   `build_query_as` 无法推断自定义的
   `content_type as "content_type: ContentType"`。改为通过 `list_by_room`
   全量查询后内存过滤，避免复杂的列映射。

3. **配额与权限协同**：上传链路需要同时检查房间配置、现有大小、token
   权限。为了确保顺序清晰，先在 `verify_room_token` 做身份校验，再用
   `ensure_permission` 比较房间与 token
   的双向权限位，最后才进行磁盘与数据库写入。

4. **测试数据回收**：集成测试里频繁创建房间、上传文件，如果不在删除流程中正确移除落盘文件，下一次跑测试就会残留垃圾数据。实现删除时，始终优先移除磁盘，再更新数据库与房间配额，确保测试幂等。

# 总结与下一步

本次迭代将房间凭证与内容链路从需求清单落地为可用的 API：

- 引入 JWT 凭证与 token 存储，支持签发、校验、撤销与列举；
- 为内容操作上锁：列表/上传/删除/下载全部强制使用
  token，并遵循房间权限与容量配额；
- 打通路由、Repository 与测试链路，确保功能可验证、可回归。

下一步可以考虑：

1. **权限自定义接口**：目前房间默认全权限，后续可新增接口允许房主配置只读或删除权限；
2. **断点续传与大文件处理**：当前上传采用全文写入，小文件
   OK，但大文件仍需优化（分块、并发 IO）；
3. **token 生命周期管理**：引入刷新机制或一次性
   token，避免长期有效带来的安全风险；
4. **审计日志**：将内容操作写入 `room_access_logs`，形成可追踪的活动记录。

通过这轮改动，房间内容的安全与可用性显著提升，也为后续细化权限、扩展审计能力打下了基础。
