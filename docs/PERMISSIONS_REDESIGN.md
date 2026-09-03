# 房间角色与权限系统重构（Authorization Redesign）

> 状态：设计定稿（开发指引）
> 范围：一次性彻底重构房间管理/权限模型 —— Role × Capability × Room 策略引擎
> 原则：长期可维护性、正确性、可演进性、简洁、模块化、可读性；不提供向后兼容债务；迭代期必须保留历史数据（sqlx migration 增量演进）。

---

## 0. TL;DR

1. **删除** `rooms.permission` 4 位掩码 + JWT `permission: u8` 内嵌副本 + `PermissionValidator/ensure_permission` 三套并存校验链。
2. **建立** 每房间独立的 `room_roles` 表：`role_key → capabilities[]`，支持 `admin / editor / reader` 三个系统角色 + 任意自定义角色。
3. **建立** `crates/board/src/authz/` 策略引擎模块：类型化 `Capability`/`Scope` 枚举 + 纯函数决策核心 `authorize()` + 带版本缓存的 `RoleTable` 加载器。业务代码只剩 `authz.require(Capability::X, resource)?`。
4. **Token 改为绑定 `role_key`**（实时角色语义：改角色矩阵立即对所有会话生效）；JWT claims 删除 `permission`，新增 `role`。
5. **新增** `room_contents.created_by_jti`，支撑 `own / any` 作用域（只能编辑/删除自己的消息与文件）。
6. **前端** 权限判定统一为 `useRoomCapabilities()` → `can(cap)` / `canOn(cap, resource)`；新增「角色与权限」配置界面（能力矩阵 + 自定义角色管理）。
7. **拒绝引入 Cedar/Casbin**（理由见 §4）；决策核心是 ~60 行纯函数，引擎接缝保留，未来出现自由策略文本需求时可无缝升级。

---

## 1. 背景与新需求说明

### 1.1 需求原文归纳

- 房间管理员与权限相关代码彻底重构，支持**细粒度配置 roles 与 role 对应的权限**。
- 房间内可配置的能力（capabilities）覆盖：
  - 房间设置（expire time / password / max entries / max size / 默认加入角色）
  - Room Permissions：share、edit、delete
  - 消息：edit、copy、delete
  - 文件：upload、delete、protect（下载保护策略/访问码）
  - 以及更多房间内的权限与能力
- 逻辑通过 **role / room / capabilities** 三要素配置、生效、校验，与业务 handler **彻底解耦**。
- **Room 是隔离单位**：角色与能力矩阵按房间独立配置（room-scoped，无全局用户体系）。
- 目标角色：`admin`、`editor`、`reader` 等，支撑更好的内容管理。
- 必须提供**优美且清晰的交互配置界面**。
- 一次重构到位（不是渐进重构），输出清晰的开发指引。

### 1.2 需求转译（规格）

| 需求 | 规格落点 |
| --- | --- |
| 细粒度角色 | `room_roles` 表，每房间一份角色矩阵；系统角色 3 个 + 自定义角色 |
| 细粒度权限 | 16 个类型化 Capability（§5），可编辑/可删除类能力带 `own/any` 作用域 |
| 彻底解耦 | 业务 handler 只调 `authz.require()`；禁止 `role == "admin"` / `can_delete()` 分支散落 |
| room 隔离 | 角色矩阵按 `room_id` 存储/加载；Principal 绑定单一 room；跨房一律 Deny |
| admin/editor/reader | 系统内置模板（§5.3），每房间可改能力集、可建自定义角色 |
| 配置界面 | 房间设置弹窗新增「角色与权限」标签页：角色列表 + 能力矩阵 + 自定义角色 CRUD |
| 一次到位 | 同一提交内：新引擎上线、旧路径全删、migration 落库、前端切换、测试齐备 |

### 1.3 非目标（明确排除）

- 不引入全局用户/账号体系（无 user 表；Principal = Room Token）。
- 不做跨房间管理（房间之间零共享权限）。
- 不做自由文本策略编辑（策略输入是结构化的能力矩阵 UI，不是 DSL）。
- 不改文件下载保护（卡密/次数）的业务语义 —— 它属于 Resource Policy 层，仅把「谁能配置它」纳入 Capability。

---

## 2. 现状梳理（重构前的事实基线）

### 2.1 权限模型

- `RoomPermission` bitflags(u8)：`VIEW_ONLY=1, EDITABLE=2, SHARE=4, DELETE=8`，serde 透明序列化为整数（`crates/board-protocol/src/models/room/permission.rs:29-34`）；`can_view/can_edit/can_share/can_delete/can_do_all` 五个判定方法（同文件 :68-82）；SQLite 存 u8、Postgres 存 i16 的 sqlx 编解码（:98-156）。
- DB 真相：`rooms.permission INTEGER NOT NULL DEFAULT 1`（`crates/board/migrations/001_initial_schema.sql:50`）。**无任何角色表**。
- 新房间默认权限 = `with_all()`（`crates/board/src/config.rs:369`），即**默认人人皆管理员**；环境变量 `ROOM_DEFAULT_PERMISSION_*` 控制默认位。

### 2.2 Token 体系

- Claims（`crates/board-protocol/src/dto/token.rs:24-44`）：`sub / room_id / room_name / permission(u8) / max_size / exp / iat / jti / token_type / refresh_jti`。权限位**内嵌**进 JWT。
- access TTL 默认 120 分钟、refresh 7 天，`exp` 被 clamp 到房间 `expire_at`（`crates/board/src/services/token.rs:15-18, 87-111`）。
- 校验链：`auth_service.rs:36-73`（decode → 类型 → jti 黑名单 → 房间存在/未过期/Open）+ `verify_token_with_room_permission` 双检查（房间位 AND token 位，`auth_service.rs:137-153`）。
- 撤销：`token_blacklist` 表按 jti；改密码整房撤销 sessions（`handlers/rooms/settings.rs:182-192`）。

### 2.3 Handler 鉴权（问题集中区）

无鉴权 middleware；三个入口并存、语义不一：

1. `AuthToken` extractor + `verify_room_token`（`handlers/token.rs:61-126`）：验证 + 双方 `can_view`。
2. `ensure_permission(claims, room_allows, action)`（`handlers/content/shared.rs:18-39`）：View/Edit/Delete 三值枚举再查双位。
3. 「DELETE 位 = 管理员」约定散布在 5 处：settings.rs:47、permissions.rs:45、tokens.rs:168/211、lifecycle.rs:128。

**已知不一致/缺陷**（重构一并消灭）：

- `chunked_upload.rs:81-86` prepare 只查 token 位不查房间位。
- policy 端点不对称：GET `/policy` 仅 View，PUT `/policy` 要求 Edit+Delete（`policy.rs:167-170`）。
- WS 与 HTTP 各自实现检查（`websocket/handler.rs:26-43`）。
- `room_contents` **没有创建者字段** —— 无法表达「只能删自己的」。
- 文档脱节：`docs/ARCHITECTURE.md` §2.2.4 仍写 3 位掩码。
- 前端死代码：`web/components/room/room-permissions.tsx`（无挂载点）；`web/api/permissionService.ts` 仅被 shareService 引用。

### 2.4 路由 → 权限映射（旧）

| 端点 | 旧要求 |
| --- | --- |
| POST/GET `/rooms/{name}` | 无 token（GET 不存在时自动建房） |
| DELETE `/rooms/{name}` | can_delete 双检查 |
| POST `/rooms/{name}/permissions` | can_delete |
| PUT `/rooms/{name}/settings` | can_delete |
| POST `/rooms/{name}/tokens` | 密码/过期/进入次数（Room Gate） |
| GET `/tokens`、DELETE `/tokens/{jti}` | can_delete |
| GET `/rooms/{name}/contents` | View |
| POST `/contents/prepare`、`/contents`、`/contents/url` | Edit |
| DELETE `/contents` | Delete |
| GET `/api/v1/contents/{id}` | View（+ ticket 策略） |
| GET/PUT `/contents/{id}/policy` | View / Edit+Delete |
| POST `/contents/{id}/generate-codes` | Edit+Delete |
| POST `/contents/{id}/redeem` | View |
| POST/GET `/messages` | Edit / View |
| WS connect | 双方 VIEW_ONLY |

### 2.5 前端现状

- 权限来源：JWT claims 位（`web/lib/utils/jwt.ts:15-99`，decode 不验签）与 `roomDetails.permissions` 求交集（`web/hooks/use-room-permissions.ts:15-27`），输出 `can{read,edit,share,delete}`。
- 硬编码位运算散布：`web/api/roomAccessService.ts:78`（`permission & 4`）、`permissionService.ts:250` 等。
- 设置 UI：`room-config-form.tsx`（过期/密码/maxViews/权限 pill），保存时先 settings 后 permissions 两个请求；`canModify = can.delete`。
- 生成类型：ts-rs + schemars 经 `cargo build -p elizabeth-board --features typescript-export` 输出到 `web/types/generated/`（`crates/board/build.rs:283-340` + `board-protocol/src/codegen.rs`）。

### 2.6 数据库与迁移机制

- 权威 schema = **Rust 侧 sqlx migrations**：`crates/board/migrations/`（SQLite）与 `migrations_pg/`（Postgres），由 `crates/board/src/db/mod.rs:66-124` 在启动时执行，迁移前自动 VACUUM INTO 备份（:126-172）。仓库中**不存在 Drizzle**。
- 现有迁移：`001_initial_schema` → `004_room_expiry_backfill` → `20260901150900_add_file_download_policy` → `20260901230000_update_file_access_codes_index`（pg 多一个 `003_datetime_columns_as_text`）。
- `v_room_summary` 视图引用 `r.permission`（`001_initial_schema.sql:439`）—— 删列必须重建视图。

---

## 3. 概念模型：三层分离

```text
┌─────────────────────────────────────────────────────────┐
│  Layer 1 · Room Gate（门禁，非授权）                       │
│  password / expire_at / max_times_entered / status       │
│  失败语义：进不了房、发不了 token（410/401/423）             │
└──────────────────────────┬──────────────────────────────┘
                           │ 通过后
                           ▼
┌─────────────────────────────────────────────────────────┐
│  Layer 2 · Authorization（本重构的核心）                   │
│  Principal(Role) × Capability × Resource → Allow | Deny  │
│  失败语义：403 AUTHZ_DENIED（稳定错误码）                   │
└──────────────────────────┬──────────────────────────────┘
                           │ Allow 后
                           ▼
┌─────────────────────────────────────────────────────────┐
│  Layer 3 · Resource Policy（业务规则，现状保留）             │
│  文件下载保护：mode(reusable/one_time)/卡密/下载次数/        │
│  上传容量预留（max_size/current_size）                     │
│  失败语义：403/410 业务错误（redeem 失败、次数用尽）           │
└─────────────────────────────────────────────────────────┘
```

**纪律：三层永不写进同一个 `if`。** Room Gate 判「能不能进门」，Authorization 判「进门后能做什么」，Resource Policy 判「这件事本身的业务规则」。

### 3.1 核心概念

| 概念 | 定义 | 载体 |
| --- | --- | --- |
| **Principal** | 持有效 Room Token 的进房身份：`{ jti, room_id, role_key }` | JWT claims `role` 字段 + `room_tokens.role_key` |
| **Role** | 房间内命名的主体分组：`{ key, display_name, capabilities[], is_system }` | `room_roles` 表，room 隔离 |
| **Capability** | 一个可鉴权的原子动作（类型化枚举，§5） | Rust enum + ts-rs TS union，同一套字符串落 DB |
| **Scope** | own-any 作用域：`Any`（任何人的资源）/ `Own`（仅自己是创建者的资源） | Grant 的第二维度，仅 3 个能力可配 |
| **Resource** | 被访问对象：`Room { room_id }` 或 `Content { room_id, content_type, created_by_jti }` | 请求时从 DB 组装 |
| **Grant** | `Capability + Scope` 的最小授权单元 | `room_roles.capabilities` JSON 数组 |

判定语义（**默认拒绝**）：

```text
Allow ⇔ ∃ grant ∈ principal 角色 grants：
          grant.capability == requested.capability
          ∧ grant.scope 覆盖请求（Any 覆盖一切；Own 仅当 resource.created_by_jti == principal.jti）
          ∧ resource.room_id == principal.room_id   （结构性强制，永远检查）
未知角色 / 空能力集 / 房间不匹配 / Own 不满足 → Deny（fail-closed）
```

---

## 4. 选型决策：自研轻量引擎，拒绝 Cedar / Casbin

参考方案建议 Cedar 首选、Casbin 备选。**本设计否决两者，采用类型化自研决策核心**，理由如下。

| 维度 | Cedar | Casbin | 自研类型化引擎（采纳） |
| --- | --- | --- | --- |
| 策略来源 | 策略文本（DSL） | model.conf + policy 行 | `room_roles` 行（UI 矩阵直接映射） |
| 本项目策略形态 | 由 DB 角色行**机器生成** Cedar PolicySet —— DSL 表达力完全用不上 | 同左，还需 adapter | 决策即「集合成员 + 作用域 + 房间匹配」，~60 行纯函数 |
| 契约保证 | schema.cedarschema 校验 | 无，靠纪律 | Rust `Capability` enum 编译期穷尽 + ts-rs 生成前端 union —— **同一个 enum 就是契约**，零漂移 |
| 依赖成本 | ~75 个传递 crate、11–17MB（[lib.rs](https://lib.rs/crates/cedar-policy)、[crates.io](https://crates.io/crates/cedar-policy)） | 成熟但引入 model/adapter 两套配置 | 0 新依赖 |
| own/any 表达 | 实体属性 when 子句（一等） | matcher 散装 | `Scope` 枚举 + `created_by_jti` 比较，一眼看到底 |
| 运行时 | 每请求构建 Entities + Authorizer | enforce((jti, dom, obj, act)) | 数组遍历，亚微秒 |
| 胶水代码 | 角色 JSON → Cedar 策略模板生成器 + 实体映射层 | 角色行 → policy 行 | 无（数据与判定同构） |
| 学习/维护成本 | DSL + 实体模型，团队需专学 | PERM 模型 | 标准 Rust 类型 |

**否决理由的本质**：本项目的策略输入是**结构化的能力矩阵 UI**（角色 → 复选框），不是管理员手写策略文本。Cedar 的核心价值（任意策略组合的表达力与形式化校验）在此场景下是投机性泛化；而它带来的代价（重依赖、DSL、机器生成策略的胶水层、双份真相：DB 角色行 vs 派生 PolicySet）恰恰违反本项目「KISS / 拒绝过度设计 / 禁止胶水层」的约束。契约层面 ts-rs 生成的前端类型已经提供比 Cedar schema 更强的保证（前后端同一枚举，编译期对齐）。

**升级触发器**（写入 ADR 备忘，出现任一即重评 Cedar）：
1. 出现按资源**属性条件**授权的硬需求（如「reader 只能下载 <10MB 文件」「仅工作时间可上传」）；
2. 出现管理员**自由编写策略文本**的产品需求；
3. 出现跨房间/全局 Principal。

**引擎接缝**：`authorize()` 是唯一的决策函数签名，handler 只依赖它。未来替换引擎 = 重写 `authz/engine.rs` + `roles.rs`，业务层零改动。

---

## 5. Capability 契约（单一真相）

### 5.1 能力全表（16 个）

Rust enum（`authz/capability.rs`）↔ serde kebab-case 字符串 ↔ ts-rs TS union ↔ `room_roles.capabilities` JSON ↔ i18n key `room.capabilities.*`。**同一套字符串，任何一端不得私造**。

| Capability | 字符串 | 可配 Scope | 含义 / 端点映射 |
| --- | --- | --- | --- |
| RoomShare | `room.share` | — | 分享链接/二维码/邀请 token（shareService、签发非默认角色 token） |
| RoomSettingsUpdate | `room.settings.update` | — | PUT `/rooms/{name}/settings`（密码/过期/容量/进入次数/默认加入角色） |
| RoomRolesManage | `room.roles.manage` | — | 角色矩阵 CRUD、GET `/tokens` 列表、DELETE `/tokens/{jti}` 撤销、以非默认角色替他人签发 |
| RoomDelete | `room.delete` | — | DELETE `/rooms/{name}` |
| MsgRead | `msg.read` | — | GET `/messages` + WS 实时接收 |
| MsgSend | `msg.send` | — | POST `/messages` |
| MsgCopy | `msg.copy` | — | 前端复制按钮（UI 能力，服务端不可强制执行） |
| MsgEdit | `msg.edit` | own/any | PUT `/contents/{id}`（content_type=0） |
| MsgDelete | `msg.delete` | own/any | DELETE `/contents`（content_type=0 条目） |
| FileList | `file.list` | — | GET `/rooms/{name}/contents` |
| FilePreview | `file.preview` | — | 内容预览 + GET `/contents/{id}/policy`（获知保护状态） |
| FileDownload | `file.download` | — | GET `/api/v1/contents/{id}` 文件流（+ ticket 链） |
| FileUpload | `file.upload` | — | POST `/contents/prepare`、`/contents`、`/contents/url`、分块上传全链 |
| FileDelete | `file.delete` | own/any | DELETE `/contents`（content_type=1/2/3 条目） |
| FilePolicyManage | `file.policy.manage` | — | PUT `/contents/{id}/policy`、POST `.../generate-codes` |

内容类型映射：`content_type=0`（文本/消息）→ `Msg*` 能力；`content_type=1/2/3`（图片/文件/URL）→ `File*` 能力。混合 id 的批量 DELETE 按条目逐一判定，任一 Deny 则整体 403。

### 5.2 Grant 序列化格式

`room_roles.capabilities` 存 JSON 字符串数组，`能力字符串[:scope]`：

```json
["msg.read", "msg.send", "msg.edit:any", "msg.delete:own", "file.upload", "file.delete:own"]
```

- 无 `:scope` 后缀 = `Any`（不可配 Scope 的能力必须省略，写入时校验）。
- 解析发生在 `RoleTable` 加载时：未知能力字符串 / 不可配 Scope 的能力带 scope / 缺后缀的 ownable 能力视为 `Any` —— 前两者 **fail-closed**（该角色视为空集 + error log）。

### 5.3 系统角色默认模板

| Capability | admin | editor | reader |
| --- | :---: | :---: | :---: |
| room.share | ✅ | — | — |
| room.settings.update | ✅ | — | — |
| room.roles.manage | ✅ | — | — |
| room.delete | ✅ | — | — |
| msg.read | ✅ | ✅ | ✅ |
| msg.send | ✅ | ✅ | — |
| msg.copy | ✅ | ✅ | ✅ |
| msg.edit | any | any | — |
| msg.delete | any | own | — |
| file.list | ✅ | ✅ | ✅ |
| file.preview | ✅ | ✅ | ✅ |
| file.download | ✅ | ✅ | ✅ |
| file.upload | ✅ | ✅ | — |
| file.delete | any | own | — |
| file.policy.manage | ✅ | — | — |

- `is_system = true` 的角色：**key 不可改、不可删除**；能力集每房间可任意修改（含 admin 自身 —— 管理员把自己锁出是允许的，UI 明确警示，可重新进房恢复）。
- 系统角色模板仅为**新建房间的种子**；一旦落库即以房间数据为准。

---

## 6. 数据模型与迁移

### 6.1 新表与列变更

```sql
-- room_roles：每房间角色矩阵（授权唯一真相）
CREATE TABLE room_roles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id      INTEGER NOT NULL REFERENCES rooms (id) ON DELETE CASCADE,
    role_key     TEXT    NOT NULL,             -- 'admin'|'editor'|'reader'|自定义
    display_name TEXT    NOT NULL,
    capabilities TEXT    NOT NULL DEFAULT '[]',-- §5.2 JSON 字符串数组
    is_system    INTEGER NOT NULL DEFAULT 0,   -- 系统角色：key 不可删
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (room_id, role_key)
);
CREATE INDEX idx_room_roles_room ON room_roles (room_id);

-- rooms：新增
ALTER TABLE rooms ADD COLUMN default_role_key TEXT NOT NULL DEFAULT 'reader';
ALTER TABLE rooms ADD COLUMN roles_version    INTEGER NOT NULL DEFAULT 1;  -- 缓存失效用
-- rooms：删除（旧模型唯一残留）
ALTER TABLE rooms DROP COLUMN permission;

-- room_tokens：token 绑定角色
ALTER TABLE room_tokens ADD COLUMN role_key TEXT;

-- room_contents：创建者归属（own 判定依据）
ALTER TABLE room_contents ADD COLUMN created_by_jti TEXT;
CREATE INDEX idx_room_contents_owner ON room_contents (room_id, created_by_jti);
```

配套改动：
- **重建 `v_room_summary`**（去掉 `r.permission` 列，`DROP VIEW` + `CREATE VIEW`）。
- Postgres 双胞胎迁移文件（`migrations_pg/`），语义一致（`is_system BOOLEAN`、JSON 存 `TEXT`）。
- 约束校验（role_key 非空、capabilities 为合法 JSON）放在**应用层写入路径**（repo 层序列化前校验），不写运行时兼容判断。

### 6.2 存量数据迁移（保留历史数据）

一条迁移内完成种子与映射（幂等语句，SQLite 用 `INSERT ... SELECT ... WHERE NOT EXISTS`）：

1. **角色种子**：为每个已有房间插入 admin/editor/reader 三行，能力集 = §5.3 模板。
2. **旧权限位 → 角色能力覆盖**（房间维度）：
   - `permission & 8 (DELETE)` → 该房 editor 角色并入 room.* 管理能力集（等价旧"全员管理员"房间）；
   - `permission & 4 (SHARE)` → editor/reader 并入 `room.share`；
   - `permission & 2 (EDITABLE)` → editor 保持默认模板（旧 edit 语义 ≈ msg.edit:any + msg.delete:own）。
3. **存量 token 映射**：`room_tokens.role_key = CASE WHEN rooms.permission & 8 THEN 'admin' WHEN & 2 THEN 'editor' ELSE 'reader' END`（保持既有会话的有效权力不缩水）。
4. `room_contents.created_by_jti` 对存量行为 NULL → Own 作用域对匿名旧内容 **Deny**（fail-closed，Any 不受影响），文档明示。

> 迁移机制遵守项目现状：**sqlx migration 文件**（SQLite + Postgres 各一），启动时自动执行、执行前自动备份（`db/mod.rs` 已内置 VACUUM INTO 备份与清理逻辑，无需重复实现）。运行时查询一律走既有 sqlx repo 模式，不写裸字符串拼接 SQL 的胶水。

### 6.3 房间创建种子

`POST /rooms/{name}`（`handlers/rooms/lifecycle.rs`）创建房间时在同一事务内：
1. 写入三行系统角色（§5.3 模板）；
2. `rooms.default_role_key = 'reader'`（可配置项，见 §7.5）；
3. 创建者 token 绑定 `role_key = 'admin'`。

> 产品决策（与旧行为的差异）：旧默认 `with_all()` = 新房间人人可删房。新默认加入角色为 **reader**（只读、由创建者 admin 分发 editor 身份码）。需要协作编辑的用户，admin 在「角色与权限」页生成并分发 editor 身份码。

---

## 7. 后端设计

### 7.1 模块结构

```text
crates/board/src/authz/
  mod.rs         # 对外仅 re-export: Capability, Scope, Grant, Authz, require
  capability.rs  # Capability/Scope/Grant 枚举 + DB JSON ↔ 类型 解析与校验（纯函数）
  engine.rs      # authorize(): 纯函数决策核心（无 IO、无时钟、可表驱动测试）
  roles.rs       # RoleTable 加载 + (room_id, roles_version) 缓存（唯一带 IO 的部分）
  guard.rs       # Authz extractor：AuthToken 校验后组装 Principal + grants；require() → AppError
```

不新增 crate；`bitflags` 依赖随 RoomPermission 一起删除。

### 7.2 决策核心（纯函数）

```rust
pub struct Principal<'a> {
    pub jti: &'a str,
    pub room_id: i64,
    pub role: &'a str,
}

pub enum Resource<'a> {
    Room { room_id: i64 },
    Content { room_id: i64, content_type: ContentType, created_by_jti: Option<&'a str> },
}

pub enum DenyReason { RoleMissing, CapabilityMissing, ScopeOwnViolation, RoomMismatch }

pub enum Decision { Allow, Deny(DenyReason) }

pub fn authorize(grants: &[Grant], principal: &Principal<'_>,
                 cap: Capability, resource: &Resource<'_>) -> Decision;
```

- `Decision` 是强类型结果（非法状态无法表达），`guard.rs` 统一映射 `Deny → 403 AppError::permission_denied`（错误码保持 `PERMISSION_DENIED`，message 附 DenyReason 便于排障）。
- 无 DB、无时钟、无随机 —— 表驱动单测矩阵直接覆盖（§10）。

### 7.3 RoleTable 加载与缓存

```rust
pub struct RoleTable { roles: HashMap<String, RoleDef>, version: u64 }
// AppState 新增：roles_cache: RwLock<HashMap<i64, (u64, Arc<RoleTable>)>>
```

- 请求路径：读 `rooms.roles_version`（已在 rooms 行内，多数请求已加载房间）→ 命中缓存直取；未命中查 `room_roles` 并回填。
- 一切角色写路径（PUT/POST/DELETE roles）**必须 bump `roles_version`**（repo 层同一事务内 `UPDATE rooms SET roles_version = roles_version + 1`），保证「改矩阵 → 全房会话立即生效」。
- 角色被删除而 token 仍引用 → 加载结果不含该 key → `RoleMissing` Deny（前端呈现「角色已被移除」，可重新进房）。

### 7.4 Token 与 Claims 变更

```rust
// RoomTokenClaims（dto/token.rs）
pub permission: u8,        // 删除
pub role: String,          // 新增：role_key 快照（仅用于展示与前端能力解析索引）
```

- **实时角色语义**：判定永远以 DB `room_roles` 为准；claims.role 只是索引。改角色矩阵 → 既有会话即时生效（无需重签）；改**某 token 的角色归属**（换角色重签/撤销）走既有 jti 撤销机制。
- 签发：`token.rs` / `refresh_token_service.rs` 写入 `room_tokens.role_key` 并进 claims；refresh 旋转时从 `room_tokens` 行读取 role_key（角色归属随 refresh 刷新，能力矩阵随每次请求实时）。
- **签发 API**：`POST /rooms/{name}/tokens` body 增 `role?: string`：
  - 缺省 = `rooms.default_role_key`；
  - 指定非默认角色：请求者须持 `RoomRolesManage`（匿名进房者只能拿默认角色）。
- 响应 DTO（TokenPair/TokenInfo）新增 `role` 与 `capabilities: Vec<Grant>`（签发时解析所得，供前端立即可用，非判定依据）。
- 存量 token：migration 已给 `room_tokens.role_key` 填值，但**旧 JWT 内无 `role` claim** —— 验签后 claims.role 缺失时以 `room_tokens.role_key` 兜底回填（仅此一处读取兼容，不构成兼容层：旧 token 自然过期，access TTL 仅 120 分钟）。

### 7.5 房间设置 API 变更

`PUT /rooms/{name}/settings`（DTO `UpdateRoomSettingsRequest`）：
- 新增 `default_role_key?: string`（必须存在于该房角色集，且不得指向已删角色）。
- 其余字段不变（password/remove_password/age_seconds/max_times_entered/max_size）。
- 所需能力：`RoomSettingsUpdate`（原为 can_delete）。

### 7.6 端点 → Capability 映射（新，全量）

| 端点 | Capability | Resource |
| --- | --- | --- |
| POST `/rooms/{name}` | —（Room Gate） | — |
| GET `/rooms/{name}` | —（公开可用性信息，不含角色矩阵） | — |
| DELETE `/rooms/{name}` | RoomDelete | Room |
| PUT `/rooms/{name}/settings` | RoomSettingsUpdate | Room |
| GET `/rooms/{name}/roles` | RoomRolesManage | Room |
| POST `/rooms/{name}/roles` | RoomRolesManage | Room |
| PUT `/rooms/{name}/roles/{key}` | RoomRolesManage | Room |
| DELETE `/rooms/{name}/roles/{key}` | RoomRolesManage（系统角色 409） | Room |
| POST `/rooms/{name}/tokens` | —（Gate；非默认角色需 RoomRolesManage） | Room |
| GET `/rooms/{name}/tokens` | RoomRolesManage | Room |
| DELETE `/rooms/{name}/tokens/{jti}` | RoomRolesManage | Room |
| POST `/rooms/{name}/permissions` | **删除**（被 roles API 取代） | — |
| GET `/rooms/{name}/contents` | FileList | Room |
| POST `/contents/prepare` `/contents` `/contents/url`（含分块全链） | FileUpload | Room |
| DELETE `/contents` | MsgDelete / FileDelete（按条目类型） | Content |
| GET `/api/v1/contents/{id}` | FilePreview（元数据）；文件流 FileDownload | Content |
| GET `/contents/{id}/policy` | FilePreview | Content |
| PUT `/contents/{id}/policy`、POST `.../generate-codes` | FilePolicyManage | Content |
| POST `/contents/{id}/redeem` | FileDownload（兑换即为了下载；策略校验仍在其后） | Content |
| PUT `/contents/{id}`（消息编辑） | MsgEdit | Content |
| POST/GET `/messages` | MsgSend / MsgRead | Room |
| WS connect + 实时事件 | MsgRead（send/send-ack 用 MsgSend） | Room |

Layer 3（Room Gate：密码/过期/次数/状态）与 Resource Policy（卡密/次数/容量预留）调用顺序与现状一致，位于 authorize 之后。

### 7.7 Handler 纪律

```rust
// 唯一合法模式
let authz = Authz::from_claims(state, &claims).await?;          // 组装 grants（缓存）
authz.require(Capability::FileUpload, &Resource::Room { room_id: room.id })?;
// ... 纯业务逻辑
```

- **禁止**：`claims.role == "admin"`、`can_delete()`、`role.contains("reader")` 等任何业务内分支（UI 展示除外）。
- **禁止**：绕过 `authz` 直接读 `room_roles` 表。
- `handlers/token.rs` 的 `verify_room_token*` 收敛为纯「身份 + Room Gate」校验（删去 can_view 位检查，能力交给 authz）。
- WS 与 HTTP 共用 `authz::guard`（`websocket/handler.rs` connect 时同一套 require）。
- codegen：`RoleDefinition` / `Grant` / `Capability` / `Scope` DTO 挂 ts_rs + JsonSchema derive，注册进 `codegen.rs` 三处注册表，`cargo build -p elizabeth-board --features typescript-export` 重新生成前端类型。

---

## 8. 前端设计

### 8.1 数据流（单一数据源）

```text
token 签发/刷新响应 ──► TokenStorage { token, role, capabilities[] }   （localStorage）
GET room (react-query ["room"]) ──► RoomView { default_role_key, roles_version, ... }
WS room_update(reason: "roles_changed") ──► invalidate ["room"] + 重签提示
                    │
                    ▼
useRoomCapabilities()   # 替代 use-room-permissions.ts
  can(cap): boolean                        # 能力级判定（UI 显隐）
  canOn(cap, {createdByJti}): boolean      # 资源级判定（own/any）
  role, capabilities, payload              # 角色名、能力集、jti（own 比较用）
```

- 判定输入 = 签发响应的 `capabilities` 快照 ∩ 房间最新矩阵（与现状交集防降级残留同构，`use-room-permissions.ts:15-27` 模式平移）。
- Own 判定需要内容归属：`Message` / `FileItem` DTO 新增 `created_by_jti`（server 端 SELECT 带出），前端 `created_by_jti === payload.jti`。

### 8.2 类型与波及替换清单

删除/重写（旧权限位在前端的全部据点）：

| 文件 | 动作 |
| --- | --- |
| `web/lib/types.ts:68,184-214`（位定义/parse/encode） | 删除，改用 generated 的 `Capability`/`Grant`/`RoleDefinition` |
| `web/lib/utils/jwt.ts:15-99` | 删位解析；保留 decode + `role` 读取 |
| `web/hooks/use-room-permissions.ts` | 重写为 `use-room-capabilities.ts` |
| `web/api/permissionService.ts` | **删除**（死重量） |
| `web/api/roomAccessService.ts:78,111`、`shareService.ts:66,114` | 改用 capabilities 判定 |
| `web/api/roomService.ts:112-136` | permissions 端点调用 → roles API |
| `web/components/room/room-permissions.tsx` | **删除**（死代码） |
| `room-config-form.tsx` | 重构为 RoomSettingsDialog（§8.3） |
| `top-bar/message-bubble/message-list/middle-column/minimal-tiptap-editor/left-sidebar/right-sidebar/file-card/file-list-view/file-preview-modal/global-file-preview-modal/room-sharing` | 权限 prop 全部切换为 `can*` 语义化结果（容器层算好，展示组件只收 props） |

### 8.3 「角色与权限」配置界面

挂载点：房间设置 Dialog（由 `room-config-form.tsx` 扩展）改为三个标签页 —— **基本设置 / 角色与权限 / 分享**。

```text
┌─ 角色与权限 ─────────────────────────────────────────────────┐
│  [+ 自定义角色]                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ● admin   系统角色                     [展开 ▾]      │   │
│  │ ┌ 房间 ──────────────────────────────────────────┐    │   │
│  │ │ [x] 分享房间 [x] 修改设置 [x] 管理角色 [x] 删除房间│    │   │
│  │ ├ 消息 ──────────────────────────────────────────┤    │   │
│  │ │ [x] 读取 [x] 发送 [x] 复制                       │    │   │
│  │ │ 编辑消息     ( ) 关闭 ( ) 仅自己 (•) 全部          │    │   │
│  │ │ 删除消息     ( ) 关闭 ( ) 仅自己 (•) 全部          │    │   │
│  │ ├ 文件 ──────────────────────────────────────────┤    │   │
│  │ │ [x] 列表 [x] 预览 [x] 下载 [x] 上传               │    │   │
│  │ │ 删除文件     ( ) 关闭 (•) 仅自己 ( ) 全部          │    │   │
│  │ │ [x] 管理下载保护                                  │    │   │
│  │ └──────────────────────────────────────────────────┘   │
│  │ ● editor  系统角色   ● reader  系统角色   ○ 审核员(自定义) │
│  └──────────────────────────────────────────────────────┘   │
│  默认加入角色： [ editor ▾ ]                                  │
└──────────────────────────────────────────────────────────────┘
```

- 能力矩阵组件 `room/roles/role-matrix.tsx`：分组（房间/消息/文件）× Checkbox；ownable 能力用三态（关闭/仅自己/全部）—— 与 `Scope::Missing/Own/Any` 同构，非法组合不可表达。
- 角色列表 `room/roles/role-list.tsx`：系统角色禁删、可改能力；自定义角色可删（删除前提示引用中的 token 数）。
- 全部走既有模式：Dialog + useMutation + `setQueryData(["room"])` + invalidate + `handleMutationError`（参照 `download-policy-dialog.tsx` / `room-config-form.tsx` 现状）。
- 入口可见性：基本设置页需 `RoomSettingsUpdate`，角色页需 `RoomRolesManage`，分享页需 `RoomShare` —— 无能力者整页隐藏。
- i18n：`web/messages/zh/room.json` 新增 `roles.*`、`capabilities.*` 命名空间（能力名与 Capability 字符串一一对应 key）；删除 `config.permissions.*`、`permissions.*`、`permissionDenied.*` 旧键。
- 视觉规范遵守 UI 约定：专业图标库图标、无 emoji。

---

## 9. 切换计划（一次切断，非渐进）

> 单一重构分支完成；合并后旧路径**不复存在**。顺序即依赖序：

| # | 交付物 | 内容 |
| --- | --- | --- |
| 1 | `authz` 模块 + 契约 | `capability.rs` / `engine.rs`（含表驱动单测）、DTO（RoleDefinition/Grant/Capability/Scope）+ codegen 注册 |
| 2 | Migration | `20260903000000_room_roles_authz.sql`（sqlite + pg 双份：建表/加列/种子映射/重建视图/删 permission 列）+ repo 层 role CRUD 与 roles_version bump |
| 3 | Token 切换 | claims `permission` → `role`；签发/refresh/响应 DTO；`room_tokens.role_key` |
| 4 | Handler 全量接入 | 按 §7.6 映射逐端点 `authz.require()`；WS 同步；Room Gate 收敛 |
| 5 | 删除旧路径 | `RoomPermission`、`permissions.rs`、`PermissionValidator`、`ensure_permission`、`config.rs` 默认位、`POST /permissions` 端点、相关旧测试 |
| 6 | 前端切换 | 生成类型重导出 → `useRoomCapabilities` → 组件 prop 切换 → 角色配置 UI → i18n |
| 7 | 测试补齐 | Rust 单测矩阵 + handler 集成 + e2e（§10） |
| 8 | 文档 | `ARCHITECTURE.md` §2.2.4 重写、`API_GUIDE_FULL.md` 端点章节重写、本文档收尾 |

**明确接受的破坏性变更**（迭代期，无兼容债务）：
- 旧 JWT（无 `role` claim）在 access TTL（120 分钟）内自然淘汰，验签时从 `room_tokens.role_key` 兜底，不写长期兼容层。
- 新房间默认加入角色由「全员满权限」改为 `editor`（§6.3）。
- `room.config.permissions` API 前端调用方一次性迁移，不保留双轨。

---

## 10. 测试策略

### 10.1 Rust 单元（`src/tests/authz/`，镜像源码组织）

表驱动矩阵（engine 纯函数，无 mock 需求）：

```text
admin × RoomDelete × Room{同房}        → Allow
reader × FileUpload                      → Deny(CapabilityMissing)
editor × MsgEdit × Content{created_by=自己, scope=own}   → Allow
editor × MsgEdit × Content{created_by=他人, scope=own}   → Deny(ScopeOwnViolation)
editor × MsgEdit × Content{created_by=他人, scope=any}   → Allow
任意角色 × 任意 × 跨房 resource           → Deny(RoomMismatch)
未知 role_key                            → Deny(RoleMissing)
Grant 解析：未知字符串 / ownable 缺 scope / 非 ownable 带 scope → fail-closed 或构造期报错
```

### 10.2 集成（沿用现有测试基建）

- 角色 CRUD API：系统角色不可删（409）、capabilities 校验失败 400、写入后 `roles_version` 递增。
- 每端点 403/200 抽样（admin 全绿、reader 全红、editor 按 §7.6）。
- 实时生效：editor 被撤 `msg.send` 后 POST /messages 即 403（无需重签）。
- Own：editor 删自己消息 200、删他人 403。
- 存量迁移：旧 permission 位房间迁移后映射正确（DELETE→admin token 等）。

### 10.3 E2E（web/e2e，screenplay 分层）

- 新增 `specs/room/roles.spec.ts`：角色矩阵编辑、自定义角色、能力随角色变化、无 RoomRolesManage 者不可见角色页。
- 改造 `remote-permissions` / `download-policy`：断言口径从「权限位撤销」改为「能力撤销」，`file.policy.manage` 只对 admin 可见。
- Screen 层新增语义化 locator（roleMatrix、roleRow(key)、capabilityCheckbox(cap)、scopeRadio(cap, scope)）。

---

## 11. 假设、局限与决策点

1. **产品决策点**（默认值已选，可调）：新房间默认加入角色 = `editor`；editor 默认 `msg.edit:any`（延续协作编辑）+ `msg.delete:own`。若希望「编辑也只限自己」，改 §5.3 模板一行即可。
2. admin 自锁（移除自身 RoomRolesManage）被允许，UI 警示；恢复途径 = 重新进房（仍持 admin token）或重建房间。
3. `msg.copy` 是 UI 能力，服务端无法强制（客户端可读即可复制），如实标注。
4. 存量内容的 `created_by_jti` 为 NULL：Own 授权对其 Deny（fail-closed），Any 授权不受影响。
5. admin API（`/admin/rooms`、`/admin/gc`）目前无鉴权 —— 本重构不覆盖（room 隔离域之外），已单独记为待办风险。
6. 性能：每请求多一次 `roles_version` 读取（多数路径已加载 rooms 行，零额外查询）；缓存失效以版本号递增，无 TTL 抖动。
7. 本文档为唯一开发指引；实现若与本契约冲突，以本文档为准并回写修订。

---

## 附：资料来源

- Cedar 依赖规模：[lib.rs/crates/cedar-policy](https://lib.rs/crates/cedar-policy)、[crates.io/crates/cedar-policy](https://crates.io/crates/cedar-policy)、[GitHub cedar-policy/cedar](https://github.com/cedar-policy/cedar)
- 现状引用全部来自仓库源码（文件:行号见 §2/§8）。
