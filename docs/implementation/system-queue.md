# 队列系统 (Queue System)

## 1. 简介

Elizabeth 队列系统采用轻量级的异步任务处理架构，基于 Tokio
的异步运行时和任务调度机制。系统当前实现主要围绕上传预留的自动清理任务，通过延时任务和后台处理确保资源的及时释放。虽然不是传统意义上的消息队列，但提供了可靠的异步任务执行、失败重试和资源清理机制，为未来扩展完整的事件驱动架构预留了接口。

主要交互方包括：

- 上传预留系统 (`crates/board/src/handlers/content.rs`) - 创建延时清理任务
- 异步任务调度器 - 基于 Tokio 的任务调度
- 资源清理服务 - 自动清理过期预留
- 后台任务管理器 - 任务生命周期管理

## 2. 数据模型

### 异步任务模型

```rust
// 当前实现中没有显式的任务模型，使用 Tokio 任务直接处理
// 计划中的任务模型结构
pub struct AsyncTask {
    pub id: String,              // 任务唯一标识
    pub task_type: TaskType,      // 任务类型枚举
    pub payload: serde_json::Value, // 任务载荷
    pub scheduled_at: NaiveDateTime, // 计划执行时间
    pub created_at: NaiveDateTime,  // 创建时间
    pub attempts: u32,            // 重试次数
    pub max_attempts: u32,        // 最大重试次数
    pub status: TaskStatus,       // 任务状态
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    CleanupExpiredReservation,    // 清理过期预留
    CleanupExpiredRoom,          // 清理过期房间
    GenerateThumbnail,           // 生成缩略图
    SendNotification,            // 发送通知
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,     // 等待执行
    Running,     // 正在执行
    Completed,   // 执行完成
    Failed,      // 执行失败
    Cancelled,   // 已取消
}
```

### 上传预留任务数据

```rust
// 上传预留清理任务 ([`crates/board/src/handlers/content.rs:242`](crates/board/src/handlers/content.rs:242))
tokio::spawn(async move {
    sleep(StdDuration::from_secs(
        UPLOAD_RESERVATION_TTL_SECONDS as u64,
    ))
    .await;
    let repo = SqliteRoomUploadReservationRepository::new(db_pool);
    if let Err(err) = repo.release_if_pending(reservation_id).await {
        log::warn!(
            "Failed to release expired reservation {}: {}",
            reservation_id,
            err
        );
    }
});
```

### 任务配置常量

```rust
// 上传预留 TTL ([`crates/board/src/handlers/content.rs:37`](crates/board/src/handlers/content.rs:37))
const UPLOAD_RESERVATION_TTL_SECONDS: i64 = 10;

// 计划中的任务配置
const DEFAULT_TASK_TIMEOUT_SECONDS: u64 = 300;
const DEFAULT_MAX_RETRY_ATTEMPTS: u32 = 3;
const TASK_RETRY_DELAY_SECONDS: u64 = 30;
```

## 3. 不变式 & 验证逻辑

### 业务规则

1. **任务唯一性**: 每个异步任务都有唯一的标识符，避免重复执行
2. **超时保护**: 所有任务都有执行超时限制，防止无限等待
3. **资源清理**: 过期的预留必须自动清理，防止资源泄漏
4. **错误隔离**: 任务执行失败不应影响主业务流程
5. **幂等性**: 任务重试时应该保证操作的幂等性

### 验证逻辑

```rust
// 预留状态验证 ([`crates/board/src/handlers/content.rs:324`](crates/board/src/handlers/content.rs:324))
let now = chrono::Utc::now().naive_utc();
if reservation.expires_at < now {
    reservation_repo
        .release_if_pending(query.reservation_id)
        .await
        .ok();
    return Err(HttpResponse::BadRequest().message("Reservation expired"));
}
```

### 任务执行约束

- **并发限制**: 同类型任务的并发执行数量限制
- **资源绑定**: 任务必须绑定到具体的资源实例
- **时间窗口**: 任务必须在指定的时间窗口内执行
- **状态一致性**: 任务状态变更必须与业务状态保持一致

## 4. 持久化 & 索引

### 当前实现策略

- **内存调度**: 当前使用 Tokio 的内存调度，无需持久化
- **数据库绑定**: 任务状态通过数据库记录间接持久化
- **故障恢复**: 服务重启后通过数据库状态重建任务

### 计划中的持久化设计

```sql
-- 异步任务表（计划中）
CREATE TABLE IF NOT EXISTS async_tasks (
    id TEXT PRIMARY KEY,
    task_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    scheduled_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    INDEX idx_async_tasks_scheduled_at (scheduled_at),
    INDEX idx_async_tasks_status (status),
    INDEX idx_async_tasks_type (task_type)
);
```

### 索引策略

- **时间索引**: 按计划执行时间索引，支持定时任务查询
- **状态索引**: 按任务状态索引，支持状态过滤查询
- **类型索引**: 按任务类型索引，支持类型分组查询

## 5. API/Handlers

### 任务管理 API（计划中）

- `POST /api/v1/admin/tasks` - 创建新任务
- `GET /api/v1/admin/tasks` - 查询任务列表
- `GET /api/v1/admin/tasks/{id}` - 获取任务详情
- `POST /api/v1/admin/tasks/{id}/cancel` - 取消任务
- `POST /api/v1/admin/tasks/{id}/retry` - 重试失败任务

### 当前任务创建接口

```rust
// 上传预留清理任务创建 ([`crates/board/src/handlers/content.rs:242`](crates/board/src/handlers/content.rs:242))
let db_pool = app_state.db_pool.clone();
tokio::spawn(async move {
    sleep(StdDuration::from_secs(
        UPLOAD_RESERVATION_TTL_SECONDS as u64,
    ))
    .await;
    let repo = SqliteRoomUploadReservationRepository::new(db_pool);
    if let Err(err) = repo.release_if_pending(reservation_id).await {
        log::warn!(
            "Failed to release expired reservation {}: {}",
            reservation_id,
            err
        );
    }
});
```

### 任务执行处理器

```rust
// 计划中的任务执行器
pub struct TaskExecutor {
    db_pool: DbPool,
    max_concurrent_tasks: usize,
    task_timeout: Duration,
}

impl TaskExecutor {
    pub async fn execute_task(&self, task: AsyncTask) -> Result<(), TaskError> {
        // 任务执行逻辑
        match task.task_type {
            TaskType::CleanupExpiredReservation => {
                self.cleanup_expired_reservation(task.payload).await
            }
            TaskType::CleanupExpiredRoom => {
                self.cleanup_expired_room(task.payload).await
            }
            // ... 其他任务类型
        }
    }
}
```

## 6. JWT 与权限

### 任务执行权限

- **系统级权限**: 任务执行通常需要系统级权限，不依赖用户 JWT
- **服务账户**: 可以考虑使用服务账户 JWT 进行任务认证
- **权限继承**: 任务继承创建者的权限范围

### 安全考虑

```rust
// 计划中的任务权限验证
pub struct TaskPermissions {
    pub can_cleanup_reservations: bool,
    pub can_cleanup_rooms: bool,
    pub can_generate_thumbnails: bool,
    pub can_send_notifications: bool,
}

impl TaskPermissions {
    pub fn for_system_admin() -> Self {
        Self {
            can_cleanup_reservations: true,
            can_cleanup_rooms: true,
            can_generate_thumbnails: true,
            can_send_notifications: true,
        }
    }
}
```

## 7. 关键代码片段

### 异步任务创建

```rust
// 上传预留清理任务 ([`crates/board/src/handlers/content.rs:242`](crates/board/src/handlers/content.rs:242))
let db_pool = app_state.db_pool.clone();
tokio::spawn(async move {
    sleep(StdDuration::from_secs(
        UPLOAD_RESERVATION_TTL_SECONDS as u64,
    ))
    .await;
    let repo = SqliteRoomUploadReservationRepository::new(db_pool);
    if let Err(err) = repo.release_if_pending(reservation_id).await {
        log::warn!(
            "Failed to release expired reservation {}: {}",
            reservation_id,
            err
        );
    }
});
```

### 资源清理逻辑

```rust
// 预留释放实现 ([`crates/board/src/handlers/content.rs:324`](crates/board/src/handlers/content.rs:324))
if reservation.expires_at < now {
    reservation_repo
        .release_if_pending(query.reservation_id)
        .await
        .ok();
    return Err(HttpResponse::BadRequest().message("Reservation expired"));
}
```

### 错误处理和日志

```rust
// 任务执行错误处理 ([`crates/board/src/handlers/content.rs:248`](crates/board/src/handlers/content.rs:248))
if let Err(err) = repo.release_if_pending(reservation_id).await {
    log::warn!(
        "Failed to release expired reservation {}: {}",
        reservation_id,
        err
    );
}
```

### 计划中的任务调度器

```rust
// 计划实现的任务调度器
pub struct TaskScheduler {
    db_pool: DbPool,
    task_rx: mpsc::Receiver<AsyncTask>,
    task_tx: mpsc::Sender<AsyncTask>,
}

impl TaskScheduler {
    pub async fn start(&self) {
        let mut task_rx = self.task_rx.clone();
        let db_pool = self.db_pool.clone();

        tokio::spawn(async move {
            while let Some(task) = task_rx.recv().await {
                let pool = db_pool.clone();
                tokio::spawn(async move {
                    let executor = TaskExecutor::new(pool);
                    if let Err(e) = executor.execute_task(task).await {
                        log::error!("Task execution failed: {}", e);
                    }
                });
            }
        });
    }
}
```

## 8. 测试要点

### 单元测试建议

- **任务创建测试**: 验证异步任务的正确创建和调度
- **资源清理测试**: 测试过期预留的自动清理机制
- **错误处理测试**: 验证任务执行失败时的错误处理
- **并发安全测试**: 测试多任务并发执行的安全性

### 集成测试建议

- **端到端任务流程**: 从任务创建到执行的完整流程
- **故障恢复测试**: 服务重启后任务状态的恢复
- **性能压力测试**: 大量并发任务的系统表现
- **资源泄漏测试**: 长时间运行下的资源使用情况

### 可靠性测试

- **任务重复执行测试**: 验证任务的幂等性
- **网络分区测试**: 模拟网络故障下的任务行为
- **数据库故障测试**: 数据库不可用时的降级处理
- **内存泄漏测试**: 长期运行的内存使用监控

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

1. **缺乏任务持久化**: 当前任务仅存在于内存中，服务重启会丢失
2. **缺乏任务监控**: 没有任务执行状态的可视化监控

### P1 优先级

1. **缺乏重试机制**: 任务执行失败后没有自动重试机制
2. **缺乏任务优先级**: 所有任务都是同等优先级，无法区分重要性

### P2 优先级

1. **缺乏分布式支持**: 当前仅支持单机部署，无法扩展到多节点
2. **缺乏任务依赖**: 无法定义任务间的依赖关系

### 未来扩展计划

```rust
// 计划中的高级队列功能
pub struct AdvancedQueueSystem {
    task_scheduler: TaskScheduler,
    priority_queue: PriorityQueue,
    dependency_manager: DependencyManager,
    monitoring: TaskMonitoring,
}

impl AdvancedQueueSystem {
    // 支持任务优先级
    pub async fn submit_priority_task(&self, task: AsyncTask, priority: TaskPriority) -> Result<(), QueueError>;

    // 支持任务依赖
    pub async fn submit_dependent_task(&self, task: AsyncTask, dependencies: Vec<String>) -> Result<(), QueueError>;

    // 支持分布式执行
    pub async fn submit_distributed_task(&self, task: AsyncTask, nodes: Vec<String>) -> Result<(), QueueError>;
}
```

## 10. 关联文档 / 代码位置

### 源码路径

- **异步任务实现**:
  [`crates/board/src/handlers/content.rs:242`](crates/board/src/handlers/content.rs:242)
- **上传预留模型**:
  [`crates/board/src/models/room/upload_reservation.rs`](crates/board/src/models/room/upload_reservation.rs)
- **预留仓库**:
  [`crates/board/src/repository/room_upload_reservation_repository.rs`](crates/board/src/repository/room_upload_reservation_repository.rs)
- **应用状态**: [`crates/board/src/state.rs`](crates/board/src/state.rs)

### 依赖配置

```toml
# 异步运行时依赖 ([`crates/board/Cargo.toml:40`](crates/board/Cargo.toml:40))
tokio = { version = "1", features = ["full"] }

# 异步工具依赖 ([`crates/board/Cargo.toml:74`](crates/board/Cargo.toml:74))
futures = "0.3"
tokio-util = { version = "0.7", features = ["io"] }

# 计划中的队列依赖
# tokio-cron-scheduler = "0.9"  # 定时任务调度
# deadpool-redis = "0.12"      # Redis 连接池
# serde_json = "1"             # 任务序列化
```

### 配置示例

```bash
# 当前配置
export UPLOAD_RESERVATION_TTL_SECONDS=10

# 计划中的队列配置
export TASK_MAX_CONCURRENT=100
export TASK_TIMEOUT_SECONDS=300
export TASK_MAX_RETRY_ATTEMPTS=3
export TASK_RETRY_DELAY_SECONDS=30

# 分布式队列配置
export QUEUE_REDIS_URL="redis://localhost:6379"
export QUEUE_WORKER_ID="worker-1"
export QUEUE_HEARTBEAT_INTERVAL=30
```

### 监控配置

```bash
# 任务监控配置
export TASK_METRICS_ENABLED=true
export TASK_METRICS_PORT=9091
export TASK_LOG_LEVEL=info

# 告警配置
export TASK_FAILURE_THRESHOLD=10
export TASK_QUEUE_SIZE_THRESHOLD=1000
export TASK_EXECUTION_TIME_THRESHOLD=300
```

### 相关文档

- [system-storage.md](system-storage.md) - 存储系统和资源清理
- [system-db.md](system-db.md) - 数据库系统和任务持久化
- [system-auth.md](system-auth.md) - 认证系统和任务权限
- [async-patterns.md](async-patterns.md) - 异步模式最佳实践（待创建）

### 扩展阅读

- Tokio 官方文档：https://tokio.rs/tokio/tutorial
- Rust 异步编程指南：https://rust-lang.github.io/async-book/
- 消息队列设计模式：https://patterns.dev/posts/message-queues/
