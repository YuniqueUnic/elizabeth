# 文件上传处理器 (Upload Handler)

## 1. 简介

文件上传处理器是 Elizabeth
系统的核心组件之一，负责处理房间内文件的上传功能。该处理器采用两阶段上传机制：首先通过预留系统确保房间容量充足，然后执行实际的文件上传。处理器支持
multipart
文件上传，包含文件类型验证、大小限制和权限检查等安全机制。主要交互方包括房间管理器、存储系统和权限验证模块。

## 2. 数据模型

### 上传预留模型 (RoomUploadReservation)

- id: i64 — 主键，预留记录的唯一标识
- room_id: i64 — 关联的房间 ID
- token_jti: String — 关联的 JWT 令牌 JTI
- file_manifest: String — 文件清单的 JSON 序列化
- reserved_size: i64 — 预留的总大小（字节）
- reserved_at: NaiveDateTime — 预留时间
- expires_at: NaiveDateTime — 预留过期时间（默认 10 秒）
- consumed_at: Option<NaiveDateTime> — 消费时间
- created_at: NaiveDateTime — 创建时间
- updated_at: NaiveDateTime — 更新时间

### 文件描述符 (UploadFileDescriptor)

- name: String — 文件名
- size: i64 — 文件大小（字节）
- mime: Option<String> — MIME 类型

### 上传响应模型

- UploadPreparationResponse: 包含预留 ID、预留大小、过期时间等信息
- UploadContentResponse: 包含上传成功的文件列表和当前房间大小

> 数据库表：`room_upload_reservations`（迁移文件：`crates/board/migrations/006_create_room_upload_reservations_table.sql`）

## 3. 不变式 & 验证逻辑

### 业务规则

- 上传前必须获得有效的房间 JWT 令牌，且令牌具有编辑权限
- 文件上传前必须通过预留系统确保房间容量充足
- 预留记录有 10 秒的 TTL，超时自动释放
- 文件名必须唯一，不允许重复上传同名文件
- 实际上传的文件必须与预留清单完全匹配（文件名、大小）
- 房间状态必须为 Open 且未过期
- 文件存储路径使用 UUID 前缀避免冲突

### TTL 时间配置

**预留 TTL 常量定义**（`crates/board/src/handlers/content.rs:36`）：

```rust
const UPLOAD_RESERVATION_TTL_SECONDS: i64 = 10;
```

**TTL 使用说明**：

- **预留有效期**：上传预留记录在创建后 10 秒内有效
- **自动清理**：系统会在 TTL 到期后自动清理未消费的预留记录
- **任务调度**：使用 `tokio::spawn` 创建异步清理任务，在 TTL 时间后执行
- **过期检查**：在上传时验证预留记录是否已过期

**TTL 计时逻辑**：

```rust
// 设置自动清理任务（第 186-189 行）
tokio::spawn(async move {
    sleep(StdDuration::from_secs(UPLOAD_RESERVATION_TTL_SECONDS as u64)).await;
    repo.release_if_pending(reservation_id).await;
});
```

**TTL 设计考虑**：

- 10 秒的 TTL 为客户端预留了足够的上传准备时间
- 自动清理机制防止预留记录长期占用系统资源
- 异步清理确保不影响主线程性能

### 验证逻辑

- 文件大小必须大于 0
- 总文件大小不能超过房间剩余容量
- 文件名经过安全过滤，防止路径遍历攻击
- MIME 类型通过文件扩展名自动检测

## 4. 持久化 & 索引

### 数据库表结构

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

### 索引设计

- `idx_room_upload_reservations_room_id`: 优化按房间查询预留记录
- `idx_room_upload_reservations_token_jti`: 优化按令牌 JTI 查询
- `idx_room_upload_reservations_expires_at`: 优化过期预留的清理

### 文件存储

- 存储根目录：`storage/rooms/{room_slug}/`
- 文件命名：`{uuid}_{sanitized_filename}`
- 使用 `sanitize_filename` crate 确保文件名安全

## 5. API/Handlers

### 预留上传空间

- **POST** `/api/v1/rooms/{name}/contents/prepare`
- 请求参数：房间名称、token、文件清单
- 响应：预留 ID、预留大小、过期时间
- 错误码：400（参数错误）、401（令牌无效）、403（权限不足）、413（容量超限）

### 执行文件上传

- **POST** `/api/v1/rooms/{name}/contents`
- 请求参数：房间名称、token、reservation_id、multipart 文件数据
- 响应：上传成功的文件列表、更新后的房间大小
- 错误码：400（预留无效）、401（令牌无效）、403（权限不足）

### 请求示例

```json
// 预留请求
POST /api/v1/rooms/myroom/contents/prepare?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
{
  "files": [
    {
      "name": "document.pdf",
      "size": 1024000,
      "mime": "application/pdf"
    }
  ]
}

// 预留响应
{
  "reservation_id": 123,
  "reserved_size": 1024000,
  "expires_at": "2023-12-01T10:00:00",
  "current_size": 512000,
  "remaining_size": 9488000,
  "max_size": 10485760
}
```

## 6. JWT 与权限

### 权限验证

- 使用 `verify_room_token` 函数验证 JWT 令牌
- 检查令牌中的 `permission` 字段是否包含编辑权限 (`can_edit()`)
- 验证令牌的 `room_id` 与目标房间匹配
- 确保令牌未被撤销且未过期

### 权限检查流程

```rust
ensure_permission(
    &verified.claims,
    verified.room.permission.can_edit(),
    ContentPermission::Edit,
)?;
```

## 7. 关键代码片段

### 预留上传空间 (crates/board/src/handlers/content.rs:172)

```rust
pub async fn prepare_upload(
    AxumPath(name): AxumPath<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UploadPreparationRequest>,
) -> HandlerResult<UploadPreparationResponse> {
    // 验证令牌和权限
    let mut verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    ensure_permission(&verified.claims, verified.room.permission.can_edit(), ContentPermission::Edit)?;

    // 计算总大小并验证文件
    let mut total_size: i64 = 0;
    let mut names = HashSet::new();
    for file in &payload.files {
        if file.size <= 0 {
            return Err(HttpResponse::BadRequest().message(format!("Invalid file size for {}", file.name)));
        }
        total_size = total_size.checked_add(file.size)
            .ok_or_else(|| HttpResponse::BadRequest().message("Total size overflow"))?;
    }

    // 创建预留记录
    let (reservation, updated_room) = reservation_repo.reserve_upload(
        &verified.room,
        &verified.claims.jti,
        &manifest_json,
        total_size,
        ttl,
    ).await?;

    // 设置自动清理任务
    tokio::spawn(async move {
        sleep(StdDuration::from_secs(UPLOAD_RESERVATION_TTL_SECONDS as u64)).await;
        repo.release_if_pending(reservation_id).await;
    });
}
```

### 执行文件上传 (crates/board/src/handlers/content.rs:286)

```rust
pub async fn upload_contents(
    AxumPath(name): AxumPath<String>,
    Query(query): Query<UploadContentQuery>,
    State(app_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> HandlerResult<UploadContentResponse> {
    // 验证预留记录
    let reservation = reservation_repo.fetch_by_id(query.reservation_id).await?
        .ok_or_else(|| HttpResponse::BadRequest().message("Reservation not found"))?;

    // 验证预留匹配
    if reservation.room_id != room_id || reservation.token_jti != verified.claims.jti {
        return Err(HttpResponse::Forbidden().message("Reservation mismatch"));
    }

    // 处理 multipart 文件
    while let Some(mut field) = multipart.next_field().await? {
        let file_name = field.file_name().ok_or_else(|| HttpResponse::BadRequest().message("File name missing"))?;
        let expected = expected_map.get(&file_name)
            .ok_or_else(|| HttpResponse::BadRequest().message("Unexpected file"))?;

        // 写入临时文件并验证大小
        let safe_file_name = sanitize_filename::sanitize(&file_name);
        let file_path = storage_dir.join(format!("{unique_segment}_{safe_file_name}"));
        // ... 文件写入逻辑
    }

    // 保存到数据库并消费预留
    let updated_room = reservation_repo.consume_reservation(
        query.reservation_id,
        room_id,
        &verified.claims.jti,
        actual_total,
        &actual_manifest_json,
    ).await?;
}
```

## 8. 测试要点

### 单元测试建议

- 测试文件大小验证逻辑（零大小、溢出）
- 测试文件名唯一性检查
- 测试预留过期机制
- 测试权限验证逻辑
- 测试文件名安全过滤

### 集成测试建议

- 完整的上传流程：预留 → 上传 → 验证
- 并发上传场景测试
- 房间容量限制测试
- 网络中断恢复测试
- 大文件上传性能测试

### 边界条件测试

- 预留刚好过期的情况
- 房间容量刚好满足的情况
- 文件名包含特殊字符的情况
- multipart 数据格式异常的情况

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

- **文件上传进度跟踪**：当前实现无法提供上传进度反馈，建议添加 WebSocket 或 SSE
  机制
- **断点续传支持**：大文件上传失败后需要重新开始，建议实现分块上传和断点续传

### P1 优先级

- **病毒扫描集成**：上传文件缺乏安全扫描，建议集成 ClamAV 或类似工具
- **文件类型验证增强**：当前仅依赖 MIME 类型检测，建议添加文件头验证

### P2 优先级

- **上传速度限制**：缺乏单个用户或房间的上传速率限制
- **存储压缩**：对于文本类文件，建议实现自动压缩以节省存储空间

## 10. 关联文档 / 代码位置

### 源码路径

- 处理器实现：`crates/board/src/handlers/content.rs:172-530`
- 路由定义：`crates/board/src/route/room.rs:28-35`
- 数据模型：`crates/board/src/models/room/upload_reservation.rs`
- 权限验证：`crates/board/src/handlers/content.rs:698-723`

### 数据库相关

- 迁移文件：`crates/board/migrations/006_create_room_upload_reservations_table.sql`
- 内容表：`crates/board/migrations/002_create_room_contents_table.sql`

### 测试文件

- 集成测试：`crates/board/tests/api_integration_tests.rs`
- 模型测试：`crates/board/src/models/room/content.rs:104-126`

### 相关文档

- [房间模型文档](model-room.md)
- [权限模型文档](model-permissions.md)
- [令牌处理器文档](handler-token.md)
