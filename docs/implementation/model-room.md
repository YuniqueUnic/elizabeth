# Room

## 1. 简介

Room 是 Elizabeth
系统的核心模型，代表一个临时的文件分享空间。系统采用"房间为中心"的设计理念，所有操作都围绕房间展开，无需传统的用户注册和登录机制。Room
模型负责管理房间的生命周期、权限控制、容量限制和访问控制。主要交互方包括房间管理相关的
handlers（`crates/board/src/handlers/rooms.rs`）、内容管理
handlers（`crates/board/src/handlers/content.rs`）以及 token 服务。

## 2. 数据模型（字段 & 类型 & 解释）

```rust
pub struct Room {
    pub id: Option<i64>,              // 主键，创建时为 None
    pub name: String,                 // 房间显示名称，唯一
    pub slug: String,                 // 房间访问标识，可用于 URL
    pub password: Option<String>,     // 房间密码（明文存储）
    pub status: RoomStatus,           // 房间状态：Open=0, Lock=1, Close=2
    pub max_size: i64,               // 最大容量限制（字节），默认 10MB
    pub current_size: i64,           // 当前已用容量（字节）
    pub max_times_entered: i64,      // 最大进入次数，默认 100
    pub current_times_entered: i64,  // 当前进入次数
    pub expire_at: Option<NaiveDateTime>, // 房间过期时间
    pub created_at: NaiveDateTime,   // 创建时间
    pub updated_at: NaiveDateTime,   // 更新时间
    pub permission: RoomPermission,  // 房间权限位标志
}
```

**RoomStatus 枚举**：

- `Open = 0`：开放状态，允许进入
- `Lock = 1`：锁定状态，不允许新进入
- `Close = 2`：关闭状态，完全不可访问

**数据库映射**：对应 `crates/board/migrations/001_create_rooms_table.sql` 中的
`rooms` 表。

## 3. 不变式 & 验证逻辑（业务规则）

- **房间唯一性**：房间名称（`name`）在系统中必须唯一，通过数据库 UNIQUE 约束保证
- **容量限制**：房间总内容大小不能超过 `max_size`（默认 10MB）
- **访问次数限制**：进入房间次数不能超过 `max_times_entered`（默认 100 次）
- **过期控制**：如果设置了 `expire_at`，超过该时间后房间不可进入
- **状态管理**：只有 `Open` 状态且未过期且未超限的房间才能进入
- **权限继承**：新创建的房间默认具有全部权限（`RoomPermission::new().with_all()`）
- **密码验证**：如果设置了密码，进入时必须提供正确密码

## 4. 持久化 & 索引（实现细节）

**数据库表结构**：

```sql
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,           -- 唯一房间名
    password TEXT,                       -- 房间密码
    status INTEGER NOT NULL DEFAULT 0,   -- 房间状态
    max_size INTEGER NOT NULL DEFAULT 10485760,  -- 10MB
    current_size INTEGER NOT NULL DEFAULT 0,
    max_times_entered INTEGER NOT NULL DEFAULT 100,
    current_times_entered INTEGER NOT NULL DEFAULT 0,
    expire_at DATETIME,                  -- 过期时间
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    permission INTEGER NOT NULL DEFAULT 1  -- 权限位标志
);
```

**索引和约束**：

- 主键：`id`（自增）
- 唯一约束：`name` 字段
- 自动触发器：更新时自动设置 `updated_at`

**ORM 使用**：使用 SQLx 的 `FromRow` trait 进行自动映射，通过
`SqliteRoomRepository` 进行数据库操作。

## 5. API/Handlers（对外行为）

**核心端点列表**：

- `POST /api/v1/rooms/{name}` - 创建房间
  - 输入：`CreateRoomParams { password: Option<String> }`
  - 输出：完整的 Room 对象
  - 错误：400（参数错误）、500（服务器错误）

- `GET /api/v1/rooms/{name}` - 查找房间
  - 输出：Room 对象（如果可进入）
  - 错误：403（房间不可进入）、500（服务器错误）

- `DELETE /api/v1/rooms/{name}` - 删除房间
  - 输出：成功消息
  - 错误：404（房间不存在）、500（服务器错误）

- `POST /api/v1/rooms/{name}/tokens` - 签发访问令牌
  - 输入：`IssueTokenRequest { password: Option<String>, token: Option<String> }`
  - 输出：`IssueTokenResponse { token: String, claims: RoomTokenClaims, expires_at: NaiveDateTime }`

- `POST /api/v1/rooms/{name}/permissions` - 更新房间权限
  - 输入：`UpdateRoomPermissionRequest { edit: bool, share: bool, delete: bool }`
  - 输出：更新后的 Room 对象

**请求/响应示例**：

```json
// 创建房间请求
POST /api/v1/rooms/myroom?password=secret123

// 响应
{
  "id": 1,
  "name": "myroom",
  "slug": "myroom",
  "password": "secret123",
  "status": 0,
  "max_size": 10485760,
  "current_size": 0,
  "max_times_entered": 100,
  "current_times_entered": 0,
  "expire_at": null,
  "created_at": "2024-01-01T00:00:00",
  "updated_at": "2024-01-01T00:00:00",
  "permission": 15
}
```

## 6. JWT 与权限（如何生成/校验）

Room 模型本身不直接处理 JWT，但提供权限基础：

- **权限来源**：房间的 `permission` 字段（`RoomPermission` 位标志）决定 JWT
  中包含的权限
- **权限传递**：JWT 签发时将 `room.permission.bits()` 存储在 token 的
  `permission` 字段中
- **权限验证**：所有操作通过验证 JWT 中的权限位来判断是否允许执行
- **管理员权限**：房间创建者可以通过持有删除权限的 JWT 来修改房间权限设置

## 7. 关键代码片段（无需粘全部，提供入口/关键函数）

**房间创建逻辑**（`crates/board/src/models/room/mod.rs:52`）：

```rust
pub fn new(name: String, password: Option<String>) -> Self {
    let now = Utc::now().naive_utc();
    Self {
        id: None,
        slug: name.clone(),
        name,
        password,
        status: RoomStatus::default(),
        max_size: MAX_ROOM_CONTENT_SIZE, // 10MB
        current_size: 0,
        max_times_entered: MAX_TIMES_ENTER_ROOM,
        current_times_entered: 0,
        expire_at: None,
        created_at: now,
        updated_at: now,
        permission: RoomPermission::new().with_all(),
    }
}
```

**房间进入验证逻辑**（`crates/board/src/models/room/mod.rs:83`）：

```rust
pub fn can_enter(&self) -> bool {
    !self.is_expired()
        && self.status() != RoomStatus::Close
        && self.current_times_entered < self.max_times_entered
}
```

**内容添加验证逻辑**（`crates/board/src/models/room/mod.rs:89`）：

```rust
pub fn can_add_content(&self, content_size: i64) -> bool {
    self.permission.can_edit() && self.current_size + content_size <= self.max_size
}
```

## 8. 测试要点（单元/集成测试建议）

- **基础功能测试**：
  - 房间创建、查找、删除的完整流程
  - 房间状态转换（Open → Lock → Close）
  - 密码验证正确和错误的情况

- **边界条件测试**：
  - 容量限制达到上限时的行为
  - 进入次数达到上限时的行为
  - 房间过期前后的访问控制

- **并发测试**：
  - 多个用户同时进入房间
  - 并发内容上传时的容量控制
  - 权限更新的原子性

- **集成测试**：
  - 创建房间 → 进入房间 → 上传文件 → 下载文件的完整流程
  - JWT 过期后的访问控制
  - 房间删除后相关数据的清理

## 9. 已知问题 / TODO / 改进建议

**P0 优先级**：

- **密码存储安全**：当前密码以明文存储，应使用 bcrypt 或 argon2 进行哈希存储
- **容量计算精度**：文本内容的容量计算使用 `text.len()`，未考虑实际编码开销

**P1 优先级**：

- **Slug 冲突处理**：当房间名称与已有 slug 冲突时，需要更智能的冲突解决策略
- **房间生命周期管理**：缺少自动清理过期房间的后台任务

**P2 优先级**：

- **访问日志增强**：记录更详细的访问信息，如 IP 地址、User-Agent 等
- **房间模板功能**：支持基于预设模板创建房间，预配置权限和限制

## 10. 关联文档 / 代码位置

**源码路径**：

- 主模型：`crates/board/src/models/room/mod.rs`
- 权限模型：`crates/board/src/models/room/permission.rs`
- 数据库迁移：`crates/board/migrations/001_create_rooms_table.sql`
- 房间处理器：`crates/board/src/handlers/rooms.rs`
- 房间仓储：`crates/board/src/repository/room_repository.rs`

**测试文件路径**：

- 单元测试：`crates/board/src/models/room/` 各模块的 `#[cfg(test)]` 块
- 集成测试：`crates/board/tests/room_repository_tests.rs`

**关联文档**：

- [model-permissions.md](./model-permissions.md) - 权限系统详细说明
- [model-session-jwt.md](./model-session-jwt.md) - JWT 令牌机制
- [model-file.md](./model-file.md) - 文件内容管理
