# 目标（一句话）

把当前 **Elizabeth**
项目（Rust、房间为中心的文件分享系统）的**现有实现**整理成一套结构化的技术说明文档库，使任何开发/运维/产品同事只通过阅读这些文档就能理解系统的模型、实现细节、handler
行为、模块交互与部署要点，便于后续维护和迭代。

# Elizabeth

Elizabeth 是一个基于 Rust

的文件分享和协作平台项目，旨在提供简单、安全、高效的文件共享解决方案。

## 项目概述

Elizabeth

项目致力于构建一个现代化的 (加密 + 限时 + 限量 + 限次)
文件分享系统，支持多种文件类型、高级安全特性。项目采用模块化设计，使用 Rust
语言确保高性能和内存安全。

## 核心架构与设计理念

本系统的核心是 **"Room"**，而不是
"User"。所有操作都围绕临时的、可配置的房间展开。

1. **无用户系统 (User-less)：** 系统不设用户注册或登录。
2. **房间即身份 (Room as Identity)：**
   "进入"一个房间（例如通过提供密码）是唯一的身份验证形式。
3. **房间管理员 (Room Admin)：**
   房间的**创建者**是该房间的唯一管理员。管理员有且仅有时限的机会设置该房间的**永久权限策略，即创建房间者的
   JWT 有效期内 (默认 7 天)**。
4. **房间访问者 (Room Visitor)：** 任何成功进入房间的后续访问者。
5. **会话凭证 (Session JWT)：**
   访问者成功进入房间后，将获颁一个短期的、特定于该房间的 JWT (JSON Web Token),
   时限 1 天。
6. **权限承载 (Embedded Permissions)：** 此 JWT 的 `payload`
   中将包含该房间的权限（由管理员设定，例如
   `READ_ONLY`）。所有后续操作（如上传、删除）都将**以此 JWT
   中携带的权限**为准，而不是实时查询数据库。

---

# 总体要求

1. **作用**：逐文件/逐模块阅读当前仓库代码，产出“当前实现 + 设计思路 +
   集成方式 + 已知问题 / 改进建议”的说明文档。
2. **覆盖面**：项目中所有核心模型（至少
   Room、Session/JWT、Permissions、File/Storage、Upload/Download、Admin/Policy、Audit/Logs、API/Handlers、Storage
   Adapter、Auth、Config、DB schema、Background
   jobs）和关键系统（网络、队列、存储、证书/加密、配置与部署）。
3. **文档形式**：每个模型/handler/系统模块生成一个独立的 Markdown 文件，放在
   `docs/implementation/` 下，按下述目录结构组织（示例）。
4. **深度与格式**：每个文档包含：模型定义（字段、类型）、数据约束、不变式（invariants）、数据库映射、handler/接口调用流程、示例请求/响应、关键代码片段（简短）、交互流程图或时序说明、测试覆盖点、已知缺陷与改进建议、对于新手的阅读顺序推荐。
5. **交付产物**：完整的 Markdown 文档集 + 每个文档对应的一个小 PR（或合并到 docs
   分支） + 一个汇总的 `architecture.md`（全局视图与拓扑图）。

---

# 推荐输出目录结构（严格遵守）

```
docs/
  implementation/
    model-room.md
    model-session-jwt.md
    model-permissions.md
    model-file.md
    handler-upload.md
    handler-download.md
    handler-admin.md
    system-storage.md
    system-db.md
    system-auth.md
    system-crypto.md
    system-queue.md
    System-1.md
    System-2.md
    architecture.md
  README.md
```

> `System-1.md` / `System-2.md` 用于写两个跨模型的系统级说明（例如：Room
> lifecycle, Data retention & purge policy, End-to-end upload pipeline).

---

# 每个模型/handler 文档模板（可复制粘贴）

下面是每个 Markdown 文件应包含的最小必填部分。把这段模板贴入
`model-xxx.md`，并按项目现状填充。

````
# <模块或模型名>（例如：Room）

## 1. 简介（1-2 段）
- 该模型在系统中的职责（简短）。
- 主要交互方（哪些 handler 或系统调用它）。

## 2. 数据模型（字段 & 类型 & 解释）
- id: i64 — 主键
- name: String — 房间名
- password: Option<String> — 房间密码（存储形式：例如 bcrypt / argon2 / plain?）
- status: i64 — 含义：0=active,1=expired...
- created_at: NaiveDateTime — 存储/序列化说明

> 如有 DB schema、SQL migration 文件路径请列出（并附上代码片段或 migration 名称）。

## 3. 不变式 & 验证逻辑（业务规则）
- 房间创建者是唯一管理员，管理员 JWT 有效期 N 天（默认 7 天）。
- 进入房间需要密码（若设）或房间不存在则返回 404。
- 权限模型（枚举/bitflags）及每个 flag 的含义。

## 4. 持久化 & 索引（实现细节）
- 数据库表名、索引、外键、唯一约束。
- ORM / SQLx / raw SQL 使用方式示例（关键代码片段）。

## 5. API/Handlers（对外行为）
- Endpoint 列表（method + path + 描述）
  - POST /rooms — 创建房间，输入字段，返回：
  - POST /rooms/{id}/enter — 验证密码并签发 Session JWT
- 每个接口的请求/响应示例（JSON），错误码说明。

## 6. JWT 与权限（如何生成/校验）
- 管理员 JWT vs 访问者 Session JWT 区别（payload、有效期、签名算法）。
- JWT payload 字段说明（must include: room_id, permissions, exp, iat, iss 等）。
- 服务器如何校验、何时拒绝。

## 7. 关键代码片段（无需粘全部，提供入口/关键函数）
```rust
// 例如：room creation handler 的关键逻辑片段
````

## 8. 测试要点（单元/集成测试建议）

- 需要覆盖的场景（创建、进入、jwt 过期、权限限制、并发进入、删除/清理）。

## 9. 已知问题 / TODO / 改进建议

- 列出 2-5 项优先级改进项（例如：密码哈希加强、jwt 存储复用、权限扩展）。

## 10. 关联文档 / 代码位置

- 源码路径：`crates/board/src/models/room.rs`、`handlers/room.rs` 等
- 测试文件路径：...

```
---

# architecture.md 内容指南（全局视角）
`architecture.md` 应包含：
- 系统概览：一句话 + 高层图（ASCII 或 Mermaid）
- 组件清单（服务、数据库、对象存储、队列、缓存）
- 数据流：上传 → 存储 → 分享（JWT）→ 下载（含权限检查）
- 部署与运维要点（配置项 env 列表、密钥管理、备份策略）
- 安全边界（加密-at-rest、TLS、JWT 签名、密码哈希策略）
- 典型时序图（上传/下载/room lifecycle）
- 依赖清单（第三方 crates、外部服务）
- 开发者快速上手（本地运行、迁移、环境变量例子）
- 常见故障与排查步骤（日志位置、常见错误码）

---

# 书写风格与工程规范（统一要求）
- 使用简洁的技术写作风格：短段、清单、示例请求/响应优先，必要时给出简短代码片段。
- 对于每个 API，必须给出：路径、方法、请求 JSON、响应 JSON、HTTP 状态码。
- 代码片段要能定位到仓库具体文件与行号（或函数名）。
- 不要假设读者对项目内部已经完全熟悉：给出“如果要读这段代码，先看 X、再看 Y”的阅读顺序提示。
- 语言：中文（技术术语保留英文），尽量统一术语（Room、Session、Visitor、Admin、Permissions）。
- 格式：Markdown，使用标题、表格与代码块；尽量避免冗长的段落。

---

# 验收标准（Deliverable Acceptance Criteria）
每个提交（PR）必须满足下列条件才能被标记为“通过”：
1. 对应 `model-*.md` 已创建并位于 `docs/implementation/`，且包含模板中所有必填项（Sections 1–10）。
2. 文档内列出的源码路径能在仓库中被检索到且准确。
3. 至少包含一个端到端示例（例如：创建房间 → 进入 → 上传文件 → 下载）并给出请求/响应示例。
4. 包含至少 2 条“已知问题 / 改进建议”及优先级标注（P0/P1/P2）。
5. 代码片段或示例请求不超过 30 行，清晰且可复制。
6. PR 标题和描述清楚说明文档覆盖哪些文件/模块，并附上 reviewer（建议：模块 owner）。

---

# 评审清单（Reviewer Checklist）
- [ ] 文档是否覆盖了该模块的所有关键职责？
- [ ] 字段类型与 DB schema 一致吗？
- [ ] 是否列出所有对外 API（method + path + 示例）？
- [ ] JWT/payload/权限的说明是否准确？
- [ ] 是否包含测试要点和已知问题？
- [ ] 文档中引用的源码路径是否存在且匹配？
- [ ] 文档语言是否清晰、无语法错误？

---

# 额外建议（可选但强烈推荐）
- 在 `docs/README.md` 放一个“阅读路线”：对新人推荐的 3 步读法（例如：先 `architecture.md`，再 `model-room.md`，最后 `handler-upload.md`）。
- 做一次 60 分钟的“文档扫盲”会议，把主要发现和跨模块问题同步给团队。
- 在每个文档开头标注最后阅读/更新时间与作者（便于后续维护）。
- 如果可能，补上简短的 Mermaid 图（如果团队支持渲染），方便理解流程。
```
