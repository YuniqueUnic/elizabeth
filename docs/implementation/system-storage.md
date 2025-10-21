# 存储系统 (Storage System)

## 1. 简介

Elizabeth
存储系统负责管理所有用户上传的文件存储，采用本地文件系统作为主要存储介质。系统以房间为单位组织文件，每个房间都有独立的存储目录，确保文件隔离和安全性。存储系统与上传预留机制紧密集成，通过预留-消费模式确保存储容量控制和并发安全。

主要交互方包括：

- 内容处理器 (`crates/board/src/handlers/content.rs`) - 处理文件上传、下载和删除
- 上传预留系统 (`crates/board/src/models/room/upload_reservation.rs`) -
  管理存储空间预留
- 房间内容仓库 (`crates/board/src/repository/room_content_repostitory.rs`) -
  持久化文件元数据

## 2. 数据模型

### 存储目录结构

```
STORAGE_ROOT/
├── {room_slug}/
│   ├── {uuid}_{sanitized_filename}
│   ├── {uuid}_{sanitized_filename}
│   └── ...
```

### 关键字段 & 类型 & 解释

- `STORAGE_ROOT: &str = "storage/rooms"` - 存储根目录常量
  ([`crates/board/src/handlers/content.rs:36`](crates/board/src/handlers/content.rs:36))
- `room_slug: String` - 房间标识符，用于创建存储目录
- `uuid: String` - 文件唯一标识符，防止文件名冲突
- `sanitized_filename: String` - 经过安全处理的文件名
- `file_path: PathBuf` - 完整的文件存储路径

### 文件元数据模型 (RoomContent)

```rust
pub struct RoomContent {
    pub id: Option<i64>,
    pub room_id: i64,
    pub content_type: ContentType,
    pub text: Option<String>,
    pub url: Option<String>,
    pub path: Option<String>,  // 文件系统路径
    pub size: Option<i64>,     // 文件大小
    pub mime_type: Option<String>, // MIME 类型
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
```

## 3. 不变式 & 验证逻辑

### 业务规则

1. **文件名安全化**: 所有文件名必须通过 `sanitize_filename::sanitize()`
   处理，防止路径遍历攻击
2. **唯一性保证**: 每个文件使用 UUID 前缀确保文件名唯一性，避免覆盖
3. **容量控制**: 文件上传前必须通过预留机制检查房间剩余容量
4. **权限验证**: 所有文件操作必须验证用户权限和房间访问权限
5. **原子性操作**: 文件写入和数据库记录必须保持一致性，失败时自动清理

### 验证逻辑

```rust
// 文件名安全化 ([`crates/board/src/handlers/content.rs:394`](crates/board/src/handlers/content.rs:394))
let safe_file_name = sanitize_filename::sanitize(&file_name);
let unique_segment = Uuid::new_v4().to_string();
let file_path = storage_dir.join(format!("{unique_segment}_{safe_file_name}"));
```

## 4. 持久化 & 索引

### 实现细节

- **存储路径**: `storage/rooms/{room_slug}/` - 基于房间 slug 的目录结构
- **文件创建**: 使用 `tokio::fs::File::create()` 异步创建文件
- **流式写入**: 使用 `tokio::io::AsyncWriteExt` 进行分块写入，支持大文件
- **MIME 检测**: 使用 `mime_guess::from_path()` 自动检测文件类型

### 关键代码片段

```rust
// 确保存储目录存在 ([`crates/board/src/handlers/content.rs:725`](crates/board/src/handlers/content.rs:725))
async fn ensure_room_storage(room_name: &str) -> Result<PathBuf, std::io::Error> {
    let safe = sanitize_filename::sanitize(room_name);
    let dir = PathBuf::from(STORAGE_ROOT).join(safe);
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}
```

### 索引策略

- 数据库索引：`room_contents.room_id` 用于快速查询房间文件
- 文件系统索引：基于目录结构的物理组织，便于批量操作

## 5. API/Handlers

### Endpoint 列表

- `GET /api/v1/rooms/{name}/contents` - 列出房间文件
- `POST /api/v1/rooms/{name}/contents/prepare` - 预留上传空间
- `POST /api/v1/rooms/{name}/contents` - 上传文件
- `DELETE /api/v1/rooms/{name}/contents` - 删除文件
- `GET /api/v1/rooms/{name}/contents/{content_id}` - 下载文件

### 上传流程

1. **预留阶段**: 客户端发送文件清单，系统预留存储空间
2. **上传阶段**: 客户端通过 multipart 上传文件，系统验证预留信息
3. **确认阶段**: 系统更新数据库记录，释放预留

### 下载流程

1. **权限验证**: 验证 JWT token 和房间访问权限
2. **文件定位**: 根据内容 ID 查询文件路径
3. **流式传输**: 使用 `ReaderStream` 进行流式文件传输

## 6. JWT 与权限

### 权限验证

存储系统通过 `ensure_permission()` 函数验证操作权限：

- **View 权限**: 允许查看和下载文件
- **Edit 权限**: 允许上传和修改文件
- **Delete 权限**: 允许删除文件

### 权限检查代码

```rust
// 权限验证 ([`crates/board/src/handlers/content.rs:705`](crates/board/src/handlers/content.rs:705))
fn ensure_permission(
    claims: &RoomTokenClaims,
    room_allows: bool,
    action: ContentPermission,
) -> Result<(), HttpResponse>
```

## 7. 关键代码片段

### 文件上传核心逻辑

```rust
// 文件写入循环 ([`crates/board/src/handlers/content.rs:401`](crates/board/src/handlers/content.rs:401))
let mut size: i64 = 0;
while let Some(chunk) = field.next().await {
    let chunk = chunk.map_err(|e| {
        HttpResponse::BadRequest().message(format!("Read upload chunk failed: {e}"))
    })?;
    size += chunk.len() as i64;
    temp_file.write_all(&chunk).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Write file failed: {e}"))
    })?;
}
```

### 文件删除与清理

```rust
// 文件删除 ([`crates/board/src/handlers/content.rs:593`](crates/board/src/handlers/content.rs:593))
for content in &contents {
    if let Some(path) = &content.path {
        fs::remove_file(path).await.ok();
    }
    freed_size += content.size.unwrap_or(0);
}
```

## 8. 测试要点

### 单元测试建议

- **文件名安全化测试**: 验证恶意路径字符串的处理
- **UUID 唯一性测试**: 确保文件名不会冲突
- **容量计算测试**: 验证文件大小计算和容量限制
- **MIME 检测测试**: 测试各种文件类型的自动识别

### 集成测试建议

- **完整上传流程测试**: 从预留到上传到确认的完整流程
- **并发上传测试**: 多用户同时上传文件的并发安全
- **权限边界测试**: 不同权限级别的操作限制
- **错误恢复测试**: 上传失败时的文件清理机制

### 性能测试

- **大文件上传测试**: 测试大文件分块上传的性能
- **并发下载测试**: 多用户同时下载的性能表现
- **存储空间压力测试**: 接近容量限制时的系统行为

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

1. **存储清理机制缺失**:
   当前没有自动清理过期房间文件的机制，建议添加定期清理任务
2. **存储空间监控**: 缺乏存储空间使用情况的监控和告警机制

### P1 优先级

1. **文件去重**: 可以考虑基于文件哈希的去重机制，节省存储空间
2. **存储分层**: 可以考虑热数据内存缓存，冷数据压缩存储

### P2 优先级

1. **分布式存储**: 当前仅支持本地存储，可扩展支持对象存储服务
2. **文件加密**: 可以添加文件内容的透明加密功能

## 10. 关联文档 / 代码位置

### 源码路径

- **存储处理器**:
  [`crates/board/src/handlers/content.rs`](crates/board/src/handlers/content.rs)
- **存储目录**: [`crates/board/storage/rooms/`](crates/board/storage/rooms/)
- **上传预留模型**:
  [`crates/board/src/models/room/upload_reservation.rs`](crates/board/src/models/room/upload_reservation.rs)
- **内容模型**:
  [`crates/board/src/models/room/content.rs`](crates/board/src/models/room/content.rs)

### 依赖配置

- **文件上传依赖**:
  [`crates/board/Cargo.toml:72-80`](crates/board/Cargo.toml:72-80)
  - `sanitize-filename = "0.6"` - 文件名安全化
  - `mime_guess = "2.0"` - MIME 类型检测
  - `tokio-util = { version = "0.7", features = ["io"] }` - 异步 IO 工具

### 配置示例

```bash
# 存储根目录环境变量
export ELIZABETH_STORAGE_ROOT="storage/rooms"

# 最大文件大小限制
export ELIZABETH_MAX_FILE_SIZE="104857600"  # 100MB

# 存储空间监控
export ELIZABETH_STORAGE_MONITORING="true"
```

### 相关文档

- [system-db.md](system-db.md) - 数据库系统和内容持久化
- [system-auth.md](system-auth.md) - 认证系统和权限管理
- [handler-upload.md](handler-upload.md) - 上传处理器详细说明
