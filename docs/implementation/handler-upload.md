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
- chunked_upload: Boolean — 是否为分块上传（默认 false）
- total_chunks: Option<i32> — 总分块数（分块上传时使用）
- uploaded_chunks: i32 — 已上传分块数（默认 0）
- file_hash: Option<String> — 文件哈希值
- chunk_size: Option<i32> — 分块大小
- upload_status: String — 上传状态（默认 'pending'）

### 分块上传模型 (RoomChunkUpload)

- id: i64 — 主键，分块记录的唯一标识
- reservation_id: i64 — 关联的预留记录 ID
- chunk_index: i32 — 分块索引
- chunk_size: i32 — 分块大小
- chunk_hash: Option<String> — 分块哈希值
- upload_status: String — 上传状态（默认 'pending'）
- created_at: NaiveDateTime — 创建时间
- updated_at: NaiveDateTime — 更新时间

### 文件描述符 (UploadFileDescriptor)

- name: String — 文件名
- size: i64 — 文件大小（字节）
- mime: Option<String> — MIME 类型

### 上传响应模型

- UploadPreparationResponse: 包含预留 ID、预留大小、过期时间等信息
- UploadContentResponse: 包含上传成功的文件列表和当前房间大小
- ChunkUploadPreparationResponse: 包含分块上传预留信息
- ChunkUploadStatusResponse: 包含分块上传进度信息

> 数据库表：`room_upload_reservations`（迁移文件：`crates/board/migrations/001_initial_schema.sql`）
> 数据库表：`room_chunk_uploads`（迁移文件：`crates/board/migrations/003_chunked_upload.sql`）

## 3. 不变式 & 验证逻辑

### 业务规则

- 上传前必须获得有效的房间 JWT 令牌，且令牌具有编辑权限
- 文件上传前必须通过预留系统确保房间容量充足
- 预留记录有 10 秒的 TTL，超时自动释放
- 文件名必须唯一，不允许重复上传同名文件
- 实际上传的文件必须与预留清单完全匹配（文件名、大小）
- 房间状态必须为 Open 且未过期
- 文件存储路径使用 UUID 前缀避免冲突
- 分块上传时，所有分块必须按顺序上传完成
- 分块上传完成后，系统自动合并文件

### TTL 时间配置

**预留 TTL 常量定义**（`crates/board/src/handlers/content.rs:37`）：

```rust
pub const DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS: i64 = 10;
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
    sleep(StdDuration::from_secs(DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS as u64)).await;
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
- 分块上传时验证分块索引和大小
- 分块哈希验证确保数据完整性

## 4. 持久化 & 索引

### 数据库表结构

```sql
-- room_upload_reservations 表
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
    chunked_upload BOOLEAN DEFAULT FALSE,
    total_chunks INTEGER,
    uploaded_chunks INTEGER DEFAULT 0,
    file_hash TEXT,
    chunk_size INTEGER,
    upload_status TEXT DEFAULT 'pending',
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

-- room_chunk_uploads 表
CREATE TABLE IF NOT EXISTS room_chunk_uploads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reservation_id INTEGER NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_size INTEGER NOT NULL,
    chunk_hash TEXT,
    upload_status TEXT NOT NULL DEFAULT 'pending',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (reservation_id) REFERENCES room_upload_reservations (id) ON DELETE CASCADE,
    UNIQUE(reservation_id, chunk_index)
);
```

### 索引设计

**room_upload_reservations 索引**：

- `idx_room_upload_reservations_room_id`: 优化按房间查询预留记录
- `idx_room_upload_reservations_token_jti`: 优化按令牌 JTI 查询
- `idx_room_upload_reservations_expires_at`: 优化过期预留的清理
- `idx_room_upload_reservations_chunked_upload`: 优化分块上传查询
- `idx_room_upload_reservations_upload_status`: 优化按状态查询

**room_chunk_uploads 索引**：

- `idx_room_chunk_uploads_reservation_id`: 优化按预留查询分块
- `idx_room_chunk_uploads_status`: 优化按状态查询分块
- `idx_room_chunk_uploads_chunk_index`: 优化按索引查询分块
- `idx_room_chunk_uploads_reservation_status`: 优化复合查询

### 文件存储

- 存储根目录：`storage/rooms/{room_slug}/`
- 文件命名：`{uuid}_{sanitized_filename}`
- 分块文件命名：`{uuid}_chunk_{index}`
- 使用 `sanitize_filename` crate 确保文件名安全

### 视图定义

```sql
-- 分块上传状态视图
CREATE VIEW IF NOT EXISTS v_chunked_upload_status AS
SELECT
    rur.id as reservation_id,
    rur.room_id,
    rur.chunked_upload,
    rur.total_chunks,
    rur.uploaded_chunks,
    rur.file_hash,
    rur.chunk_size,
    rur.upload_status,
    rur.expires_at,
    CASE
        WHEN rur.total_chunks IS NULL THEN 0.0
        WHEN rur.total_chunks = 0 THEN 0.0
        ELSE CAST(rur.uploaded_chunks AS REAL) / rur.total_chunks * 100
    END as upload_progress,
    COUNT(rcu.id) as total_uploaded_chunks,
    COUNT(CASE WHEN rcu.upload_status = 'uploaded' THEN 1 END) as verified_chunks
FROM room_upload_reservations rur
LEFT JOIN room_chunk_uploads rcu ON rur.id = rcu.reservation_id
WHERE rur.chunked_upload = TRUE
GROUP BY rur.id;
```

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

### 准备分块上传

- **POST** `/api/v1/rooms/{name}/uploads/chunks/prepare`
- 请求参数：房间名称、token、文件信息（大小、分块大小）
- 响应：预留 ID、总分块数、分块大小
- 错误码：400（参数错误）、401（令牌无效）、403（权限不足）、413（容量超限）

### 上传分块

- **POST** `/api/v1/rooms/{name}/uploads/chunks`
- 请求参数：房间名称、token、reservation_id、chunk_index、分块数据
- 响应：分块上传成功确认
- 错误码：400（分块无效）、401（令牌无效）、403（权限不足）

### 查询上传状态

- **GET** `/api/v1/rooms/{name}/uploads/chunks/status`
- 请求参数：房间名称、token、reservation_id
- 响应：上传进度、已上传分块数、总分块数
- 错误码：400（预留无效）、401（令牌无效）

### 完成文件合并

- **POST** `/api/v1/rooms/{name}/uploads/chunks/complete`
- 请求参数：房间名称、token、reservation_id
- 响应：合并完成确认、文件信息
- 错误码：400（分块不完整）、401（令牌无效）、403（权限不足）

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

// 分块上传准备请求
POST /api/v1/rooms/myroom/uploads/chunks/prepare?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
{
  "file_size": 10485760,
  "chunk_size": 1048576,
  "file_name": "large_file.zip"
}

// 分块上传准备响应
{
  "reservation_id": 456,
  "total_chunks": 10,
  "chunk_size": 1048576,
  "expires_at": "2023-12-01T10:00:00"
}

// 上传状态查询响应
{
  "reservation_id": 456,
  "upload_progress": 60.0,
  "uploaded_chunks": 6,
  "total_chunks": 10,
  "upload_status": "uploading"
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
        sleep(StdDuration::from_secs(DEFAULT_UPLOAD_RESERVATION_TTL_SECONDS as u64)).await;
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

### 准备分块上传 (crates/board/src/handlers/chunked_upload.rs:45)

```rust
pub async fn prepare_chunked_upload(
    AxumPath(name): AxumPath<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<ChunkedUploadPreparationRequest>,
) -> HandlerResult<ChunkedUploadPreparationResponse> {
    // 验证令牌和权限
    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    ensure_permission(&verified.claims, verified.room.permission.can_edit(), ContentPermission::Edit)?;

    // 计算分块信息
    let total_chunks = (payload.file_size + payload.chunk_size - 1) / payload.chunk_size;

    // 创建分块上传预留
    let reservation = reservation_repo.create_chunked_reservation(
        &verified.room,
        &verified.claims.jti,
        payload.file_size,
        payload.chunk_size,
        total_chunks,
        &payload.file_name,
    ).await?;

    ChunkedUploadPreparationResponse {
        reservation_id: reservation.id,
        total_chunks,
        chunk_size: payload.chunk_size,
        expires_at: reservation.expires_at,
    }
}
```

### 上传分块 (crates/board/src/handlers/chunked_upload.rs:89)

```rust
pub async fn upload_chunk(
    AxumPath(name): AxumPath<String>,
    Query(query): Query<ChunkUploadQuery>,
    State(app_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> HandlerResult<ChunkUploadResponse> {
    // 验证预留记录
    let reservation = reservation_repo.fetch_by_id(query.reservation_id).await?
        .ok_or_else(|| HttpResponse::BadRequest().message("Reservation not found"))?;

    // 验证分块索引
    if query.chunk_index < 0 || query.chunk_index >= reservation.total_chunks.unwrap_or(0) {
        return Err(HttpResponse::BadRequest().message("Invalid chunk index"));
    }

    // 处理分块数据
    while let Some(mut field) = multipart.next_field().await? {
        let chunk_data = field.bytes().await?;

        // 验证分块大小
        if chunk_data.len() != reservation.chunk_size.unwrap_or(0) as usize {
            // 最后一个分块可能小于标准分块大小
            if query.chunk_index != reservation.total_chunks.unwrap_or(0) - 1 {
                return Err(HttpResponse::BadRequest().message("Invalid chunk size"));
            }
        }

        // 保存分块文件
        let chunk_path = storage_dir.join(format!("chunk_{}_{}", reservation.id, query.chunk_index));
        tokio::fs::write(&chunk_path, &chunk_data).await?;

        // 创建分块记录
        chunk_repo.create_chunk_record(
            reservation.id,
            query.chunk_index,
            chunk_data.len() as i32,
            Some(calculate_hash(&chunk_data)),
        ).await?;

        break;
    }

    ChunkUploadResponse {
        chunk_index: query.chunk_index,
        uploaded: true,
    }
}
```

## 8. 测试要点

### 单元测试建议

- 测试文件大小验证逻辑（零大小、溢出）
- 测试文件名唯一性检查
- 测试预留过期机制
- 测试权限验证逻辑
- 测试文件名安全过滤
- 测试分块上传逻辑
- 测试分块合并逻辑
- 测试上传进度计算

### 集成测试建议

- 完整的上传流程：预留 → 上传 → 验证
- 分块上传流程：准备 → 分块上传 → 状态查询 → 合并
- 并发上传场景测试
- 房间容量限制测试
- 网络中断恢复测试
- 大文件上传性能测试

### 边界条件测试

- 预留刚好过期的情况
- 房间容量刚好满足的情况
- 文件名包含特殊字符的情况
- multipart 数据格式异常的情况
- 分块上传中部分分块失败的情况
- 分块索引重复或缺失的情况

## 9. 已实现功能

### 已完成功能

- ✅ 两阶段上传机制（预留 + 上传）
- ✅ 文件大小和容量限制验证
- ✅ JWT 权限验证
- ✅ 文件名安全过滤
- ✅ 分块上传支持
- ✅ 分块上传进度跟踪
- ✅ 自动文件合并
- ✅ 预留记录自动清理
- ✅ 分块哈希验证
- ✅ 上传状态查询

### 计划中功能

- 🔄 文件上传进度实时推送（WebSocket/SSE）
- 🔄 病毒扫描集成
- 🔄 文件类型验证增强
- 🔄 上传速度限制
- 🔄 存储压缩

## 10. 关联文档 / 代码位置

### 源码路径

- 普通上传处理器实现：`crates/board/src/handlers/content.rs:172-530`
- 分块上传处理器实现：`crates/board/src/handlers/chunked_upload.rs`
- 路由定义：`crates/board/src/route/room.rs:28-35`
- 数据模型：`crates/board/src/models/room/upload_reservation.rs`
- 分块模型：`crates/board/src/models/room/chunk_upload.rs`
- 权限验证：`crates/board/src/handlers/content.rs:698-723`

### 数据库相关

- 迁移文件：`crates/board/migrations/001_initial_schema.sql`
- 刷新令牌迁移：`crates/board/migrations/002_refresh_tokens.sql`
- 分块上传迁移：`crates/board/migrations/003_chunked_upload.sql`
- 内容表：`crates/board/migrations/001_initial_schema.sql`

### 测试文件

- 集成测试：`crates/board/tests/api_integration_tests.rs`
- 模型测试：`crates/board/src/models/room/content.rs:104-126`

### 相关文档

- [房间模型文档](model-room.md)
- [权限模型文档](model-permissions.md)
- [令牌处理器文档](handler-token.md)
- [刷新令牌处理器文档](handler-refresh-token.md)
- [分块上传设计文档](chunked-upload-design.md)
- [分块上传 API 文档](chunked-upload-api.md)
