# 数据库实现进度文档

## 概述

本文档记录了 Elizabeth
项目中数据库模块的实现进度，包括已完成的任务、遇到的挑战以及解决方案。

## 已完成的任务

### 1. 依赖项配置 ✅

- 更新了 `crates/board/Cargo.toml`
- 添加了 `sqlx` 的 `sqlite` 和 `migrate` 特性
- 添加了 `async-trait` 依赖
- 添加了 `chrono` 的 `serde` 特性

### 2. 数据库模块结构 ✅

- 创建了 `crates/board/src/db/` 目录
- 实现了 `db/mod.rs` 模块
- 定义了 `DbPool` 类型别名

### 3. 数据库迁移文件 ✅

- 创建了 `migrations/` 目录
- 实现了以下迁移文件：
  - `001_create_rooms_table.sql` - 创建房间表
  - `002_create_room_contents_table.sql` - 创建房间内容表
  - `003_create_room_access_logs_table.sql` - 创建房间访问日志表
  - `004_add_indexes.sql` - 添加索引

### 4. 数据模型 ✅

- 更新了 `Room` 结构体，添加数据库相关字段和序列化支持
- 添加了 `RoomContent` 结构体
- 添加了 `RoomAccessLog` 结构体
- 实现了相关的枚举类型：`RoomStatus`、`ContentType`、`AccessAction`

### 5. Repository 模式实现 ✅

- 实现了 `RoomRepository` trait
- 实现了 `SqliteRoomRepository` 具体实现
- 包含以下方法：
  - `exists()` - 检查房间是否存在
  - `create()` - 创建房间
  - `find_by_name()` - 按名称查找房间
  - `find_by_id()` - 按 ID 查找房间
  - `update()` - 更新房间信息
  - `delete()` - 删除房间
  - `list_expired()` - 列出过期房间

### 6. HTTP 处理函数 ✅

- 实现了 `create()` - 创建房间处理函数
- 实现了 `find()` - 查找房间处理函数
- 实现了 `delete()` - 删除房间处理函数
- 实现了 `api_router()` - API 路由器

### 7. 应用集成 ✅

- 修改了 `lib.rs` 集成数据库到应用启动流程
- 添加了数据库初始化和迁移逻辑

## 当前状态

### 编译状态 ⚠️

当前代码存在一些编译问题，主要是：

1. SQLx 查询宏的参数问题
2. 路由宏的使用问题

### 已知问题

1. **SQLx 查询宏参数问题**：需要正确绑定查询参数
2. **路由宏使用问题**：需要正确导入和使用 `routes!` 宏
3. **数据库配置**：需要从配置文件中读取数据库 URL

## 下一步计划

1. 修复 SQLx 查询宏的参数绑定问题
2. 修复路由宏的使用问题
3. 完善数据库配置
4. 添加单元测试
5. 添加集成测试

## 技术细节

### 数据库表结构

#### rooms 表

```sql
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    password TEXT,
    status INTEGER NOT NULL DEFAULT 0,
    max_size INTEGER NOT NULL DEFAULT 10485760,
    current_size INTEGER NOT NULL DEFAULT 0,
    max_times_entered INTEGER NOT NULL DEFAULT 100,
    current_times_entered INTEGER NOT NULL DEFAULT 0,
    expire_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    allow_edit BOOLEAN NOT NULL DEFAULT TRUE,
    allow_download BOOLEAN NOT NULL DEFAULT TRUE,
    allow_preview BOOLEAN NOT NULL DEFAULT TRUE
);
```

#### room_contents 表

```sql
CREATE TABLE IF NOT EXISTS room_contents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    content_type INTEGER NOT NULL,
    content_data TEXT NOT NULL,
    file_name TEXT,
    file_size INTEGER,
    file_path TEXT,
    mime_type TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

#### room_access_logs 表

```sql
CREATE TABLE IF NOT EXISTS room_access_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    access_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action INTEGER NOT NULL,
    details TEXT,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

### Repository 模式

我们使用了 Repository 模式来抽象数据库访问，这提供了以下好处：

1. 可测试性 - 可以轻松模拟 Repository 进行单元测试
2. 可扩展性 - 可以轻松切换不同的数据库实现
3. 关注点分离 - 业务逻辑与数据访问逻辑分离

### 错误处理

所有数据库操作都返回 `Result<T>` 类型，使用 `anyhow::Result`
进行错误处理，这样可以：

1. 提供详细的错误信息
2. 简化错误传播
3. 支持自定义错误类型

## 总结

数据库模块的实现已经基本完成，主要功能已经实现。目前需要解决一些编译问题，然后就可以进行测试和部署。整个实现遵循了
Rust 的最佳实践，包括所有权、借用检查和类型安全。

---

已在仓库根目录执行
`DATABASE_URL="sqlite:./crates/board/database.db" cargo sqlx prepare --workspace`，成功生成
.sqlx/ query-*.json 元数据，sqlx 宏不再报“unable to open database
file”。问题原因是：cargo sqlx 进程工作目录在工作区根，因此 URL
中的相对路径需指向 `crates/board/database.db`（或提供绝对路径），之前的
sqlite:database.db 指向了不存在的根目录文件。后续 建议：

- 在仓库根目录运行相关命令，或使用 .env 中的
  DATABASE_URL=sqlite:./crates/board/database.db（无需手动覆盖）。
- 如需重新准备，可重复上述命令；若希望使用其他路径，确保提前运行迁移并同步更新
  DATABASE_URL。
- .sqlx 文件夹应纳入版本控制，保证 CI/离线编译与本地一致。
