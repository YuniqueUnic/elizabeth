# 数据库系统 (Database System)

## 1. 简介

Elizabeth 数据库系统基于 SQLite 构建，采用 WAL (Write-Ahead Logging)
模式提供高并发读写性能。系统使用 SQLx 作为异步数据库驱动，支持编译时 SQL
验证和类型安全的查询。数据库设计以房间为中心，包含房间管理、内容存储、访问日志、令牌管理和上传预留等核心功能模块。

主要交互方包括：

- 应用服务层 (`crates/board/src/services/`) - 业务逻辑处理
- 仓库层 (`crates/board/src/repository/`) - 数据访问抽象
- 处理器层 (`crates/board/src/handlers/`) - HTTP 请求处理
- 迁移系统 (`crates/board/migrations/`) - 数据库版本管理

## 2. 数据模型

### 核心表结构

#### rooms 表 - 房间信息

```sql
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL DEFAULT '',
    password TEXT,
    status INTEGER NOT NULL DEFAULT 0, -- 0: Open, 1: Lock, 2: Close
    max_size INTEGER NOT NULL DEFAULT 10485760, -- 默认 10MB
    current_size INTEGER NOT NULL DEFAULT 0,
    max_times_entered INTEGER NOT NULL DEFAULT 100,
    current_times_entered INTEGER NOT NULL DEFAULT 0,
    expire_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    permission INTEGER NOT NULL DEFAULT 1 -- 权限位掩码
);
```

#### room_contents 表 - 房间内容

```sql
CREATE TABLE IF NOT EXISTS room_contents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    content_type INTEGER NOT NULL, -- 0: text, 1: image, 2: file
    text TEXT,
    url TEXT,
    path TEXT,
    size INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

#### room_access_logs 表 - 访问日志

```sql
CREATE TABLE IF NOT EXISTS room_access_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    access_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action INTEGER NOT NULL, -- 0: enter, 1: exit, 2: create_content, 3: delete_content
    details TEXT,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

#### room_tokens 表 - 令牌管理

```sql
CREATE TABLE IF NOT EXISTS room_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    jti TEXT NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    revoked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

#### room_upload_reservations 表 - 上传预留

```sql
CREATE TABLE IF NOT EXISTS room_upload_reservations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    token_jti TEXT NOT NULL,
    file_manifest TEXT NOT NULL,
    reserved_size INTEGER NOT NULL,
    reserved_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    consumed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

### 字段类型映射

- `INTEGER` - i64 数值类型
- `TEXT` - String 字符串类型
- `DATETIME` - NaiveDateTime 时间类型
- `FOREIGN KEY` - 外键约束，支持级联删除

## 3. 不变式 & 验证逻辑

### 业务规则

1. **房间唯一性**: `rooms.name` 和 `rooms.slug` 必须唯一
2. **容量约束**: `rooms.current_size` 不能超过 `rooms.max_size`
3. **访问次数限制**: `rooms.current_times_entered` 不能超过
   `rooms.max_times_entered`
4. **外键完整性**: 所有外键引用必须有效，支持级联删除
5. **时间一致性**: `created_at` ≤ `updated_at`，`reserved_at` ≤ `expires_at`

### 数据完整性约束

```sql
-- 唯一约束
CREATE UNIQUE INDEX IF NOT EXISTS idx_rooms_slug ON rooms(slug);

-- 外键约束
FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE

-- 检查约束 (应用层实现)
-- rooms.current_size <= rooms.max_size
-- rooms.current_times_entered <= rooms.max_times_entered
```

### 触发器

```sql
-- 自动更新时间戳触发器
CREATE TRIGGER IF NOT EXISTS update_rooms_updated_at
    AFTER UPDATE ON rooms
    FOR EACH ROW
BEGIN
    UPDATE rooms SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
```

## 4. 持久化 & 索引

### 索引策略

```sql
-- 房间表索引
CREATE INDEX IF NOT EXISTS idx_rooms_name ON rooms(name);
CREATE INDEX IF NOT EXISTS idx_rooms_status ON rooms(status);
CREATE INDEX IF NOT EXISTS idx_rooms_expire_at ON rooms(expire_at);
CREATE INDEX IF NOT EXISTS idx_rooms_created_at ON rooms(created_at);

-- 内容表索引
CREATE INDEX IF NOT EXISTS idx_room_contents_room_id ON room_contents(room_id);
CREATE INDEX IF NOT EXISTS idx_room_contents_content_type ON room_contents(content_type);
CREATE INDEX IF NOT EXISTS idx_room_contents_created_at ON room_contents(created_at);

-- 访问日志索引
CREATE INDEX IF NOT EXISTS idx_room_access_logs_room_id ON room_access_logs(room_id);
CREATE INDEX IF NOT EXISTS idx_room_access_logs_access_time ON room_access_logs(access_time);
CREATE INDEX IF NOT EXISTS idx_room_access_logs_action ON room_access_logs(action);

-- 令牌表索引
CREATE INDEX IF NOT EXISTS idx_room_tokens_room_id ON room_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_tokens_expires_at ON room_tokens(expires_at);

-- 上传预留索引
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_room_id ON room_upload_reservations(room_id);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_token_jti ON room_upload_reservations(token_jti);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_expires_at ON room_upload_reservations(expires_at);
```

### 连接池配置

```rust
// 数据库连接池配置 ([`crates/board/src/db/mod.rs:26`](crates/board/src/db/mod.rs:26))
let pool = SqlitePoolOptions::new()
    .max_connections(20)        // 最大连接数
    .min_connections(5)         // 最小连接数
    .acquire_timeout(Duration::from_secs(30))  // 获取连接超时
    .idle_timeout(Duration::from_secs(600))    // 空闲超时
    .max_lifetime(Duration::from_secs(1800))   // 连接最大生命周期
    .connect_with(connect_options)
    .await?;
```

### WAL 模式配置

```rust
// SQLite WAL 模式配置 ([`crates/board/src/db/mod.rs:20`](crates/board/src/db/mod.rs:20))
let connect_options = SqliteConnectOptions::from_str(database_url)?
    .create_if_missing(true)
    .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)  // WAL 模式
    .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
    .busy_timeout(std::time::Duration::from_secs(30));
```

## 5. API/Handlers

### 数据库初始化 API

```rust
// 初始化数据库连接池 ([`crates/board/src/db/mod.rs:17`](crates/board/src/db/mod.rs:17))
pub async fn init_db(database_url: &str) -> Result<DbPool>

// 运行数据库迁移 ([`crates/board/src/db/mod.rs:40`](crates/board/src/db/mod.rs:40))
pub async fn run_migrations(pool: &DbPool) -> Result<()>
```

### 迁移管理

- **自动迁移**: 系统启动时自动执行 `sqlx::migrate!("./migrations")`
- **版本控制**: 通过迁移文件名管理版本，当前使用单一初始架构文件
- **回滚支持**: 当前实现不支持自动回滚，需要手动处理

#### 迁移文件结构

项目已合并为单一初始架构迁移文件：

- [`001_initial_schema.sql`](crates/board/migrations/001_initial_schema.sql) -
  初始数据库架构
  - 包含所有表结构创建
  - 包含所有索引创建
  - 包含所有触发器创建
  - 按照依赖关系正确排序：rooms → room_contents → room_tokens →
    room_upload_reservations → room_access_logs → 索引 → 触发器

#### 迁移历史

原多个迁移文件已合并为单一文件：

- ~~0001_create_rooms.sql~~ → 合并到 001_initial_schema.sql
- ~~0002_create_room_contents.sql~~ → 合并到 001_initial_schema.sql
- ~~0003_create_room_access_logs.sql~~ → 合并到 001_initial_schema.sql
- ~~0004_create_room_tokens.sql~~ → 合并到 001_initial_schema.sql
- ~~0005_create_room_upload_reservations.sql~~ → 合并到 001_initial_schema.sql
- ~~9999_create_indexes.sql~~ → 合并到 001_initial_schema.sql

这种合并方式适合开发阶段，可以从零开始构建数据库。

### 查询模式

```rust
// 类型安全查询示例
let room = sqlx::query_as!(
    Room,
    "SELECT * FROM rooms WHERE slug = ? AND status != ?",
    room_slug,
    RoomStatus::Close as i64
)
.fetch_optional(&pool)
.await?;
```

## 6. JWT 与权限

### 权限存储

权限信息以位掩码形式存储在 `rooms.permission` 字段中：

```rust
// 权限位定义 ([`crates/board/src/models/room/permission.rs:30`](crates/board/src/models/room/permission.rs:30))
pub struct RoomPermission: u8 {
    const VIEW_ONLY = 1;    // 查看权限
    const EDITABLE = 1 << 1; // 编辑权限
    const SHARE = 1 << 2;   // 分享权限
    const DELETE = 1 << 3;  // 删除权限
}
```

### 令牌持久化

JWT 令牌的 JTI (JWT ID) 存储在 `room_tokens` 表中，用于：

- 令牌撤销管理
- 防止重复签发
- 审计追踪

## 7. 关键代码片段

### 数据库初始化

```rust
// 数据库连接池初始化 ([`crates/board/src/db/mod.rs:17`](crates/board/src/db/mod.rs:17))
pub async fn init_db(database_url: &str) -> Result<DbPool> {
    info!("初始化数据库连接池：{}", database_url);

    let connect_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(30));

    let pool = SqlitePoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect_with(connect_options)
        .await?;

    info!("数据库连接池初始化成功");
    Ok(pool)
}
```

### 迁移执行

```rust
// 运行数据库迁移 ([`crates/board/src/db/mod.rs:40`](crates/board/src/db/mod.rs:40))
pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    info!("开始运行数据库迁移");

    match sqlx::migrate!("./migrations").run(pool).await {
        Ok(_) => {
            info!("数据库迁移完成");
            Ok(())
        }
        Err(e) => {
            error!("数据库迁移失败：{}", e);
            Err(anyhow::anyhow!("数据库迁移失败：{}", e))
        }
    }
}
```

### 配置结构

```rust
// 数据库配置结构 ([`crates/board/src/db/mod.rs:58`](crates/board/src/db/mod.rs:58))
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}
```

## 8. 测试要点

### 单元测试建议

- **连接池测试**: 验证连接池的创建、获取和释放
- **迁移测试**: 测试数据库迁移的正确性和回滚能力
- **约束测试**: 验证外键约束和唯一约束的有效性
- **索引性能测试**: 测试索引对查询性能的提升

### 集成测试建议

- **并发读写测试**: 多线程同时读写数据库的安全性
- **事务隔离测试**: 验证事务的 ACID 特性
- **连接泄漏测试**: 长时间运行下的连接池稳定性
- **数据一致性测试**: 复杂业务场景下的数据一致性

### 压力测试

- **高并发连接**: 测试最大连接数限制下的系统表现
- **大量数据插入**: 测试大量数据插入的性能和稳定性
- **复杂查询性能**: 测试复杂查询的响应时间

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

1. **备份策略缺失**: 当前没有自动数据库备份机制，建议添加定期备份
2. **监控指标不足**: 缺乏数据库性能监控和告警机制

### P1 优先级

1. **连接池优化**: 可以根据实际负载动态调整连接池大小
2. **查询优化**: 可以添加查询执行计划分析和慢查询日志

### P2 优先级

1. **读写分离**: 可以考虑读副本配置，提升读性能
2. **数据库分片**: 当数据量增长时，可以考虑水平分片

## 10. 关联文档 / 代码位置

### 源码路径

- **数据库模块**: [`crates/board/src/db/mod.rs`](crates/board/src/db/mod.rs)
- **迁移文件**: [`crates/board/migrations/`](crates/board/migrations/)
- **房间模型**:
  [`crates/board/src/models/room/mod.rs`](crates/board/src/models/room/mod.rs)
- **权限模型**:
  [`crates/board/src/models/room/permission.rs`](crates/board/src/models/room/permission.rs)

### 迁移文件列表

- [`001_initial_schema.sql`](crates/board/migrations/001_initial_schema.sql) -
  初始数据库架构（合并所有表、索引和触发器）

#### 历史迁移文件（已合并）

以下文件已合并到 `001_initial_schema.sql` 中：

- ~~0001_create_rooms.sql~~ - 创建房间表
- ~~0002_create_room_contents.sql~~ - 创建内容表
- ~~0003_create_room_access_logs.sql~~ - 创建访问日志表
- ~~0004_create_room_tokens.sql~~ - 创建令牌表
- ~~0005_create_room_upload_reservations.sql~~ - 创建上传预留表
- ~~9999_create_indexes.sql~~ - 添加索引

### 依赖配置

```toml
# 数据库依赖 ([`crates/board/Cargo.toml:60-66`](crates/board/Cargo.toml:60-66))
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "tls-rustls-aws-lc-rs",
    "sqlite",
    "migrate",
    "chrono",
] }
```

### 配置示例

```bash
# 数据库连接配置
export DATABASE_URL="sqlite:app.db"

# 连接池配置
export DB_MAX_CONNECTIONS=20
export DB_MIN_CONNECTIONS=5
export DB_ACQUIRE_TIMEOUT=30
export DB_IDLE_TIMEOUT=600
export DB_MAX_LIFETIME=1800

# WAL 模式配置
export SQLITE_JOURNAL_MODE=WAL
export SQLITE_SYNCHRONOUS=NORMAL
```

### 相关文档

- [system-storage.md](system-storage.md) - 存储系统和文件管理
- [system-auth.md](system-auth.md) - 认证系统和令牌管理
- [model-room.md](model-room.md) - 房间模型详细说明
