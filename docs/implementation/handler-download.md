# 文件下载处理器 (Download Handler)

## 1. 简介

文件下载处理器是 Elizabeth
系统的核心组件之一，负责处理房间内文件和内容的下载功能。该处理器支持多种内容类型的流式下载，包括文件、图片、文本和
URL。处理器实现了严格的权限检查机制，确保只有具有查看权限的用户才能访问内容。主要交互方包括房间管理器、存储系统、权限验证模块和访问日志记录系统。

## 2. 数据模型

### 房间内容模型 (RoomContent)

- id: Option<i64> — 主键，内容记录的唯一标识
- room_id: i64 — 关联的房间 ID
- content_type: ContentType — 内容类型（Text=0, Image=1, File=2, Url=3）
- text: Option<String> — 文本内容
- url: Option<String> — URL 链接
- path: Option<String> — 服务器磁盘上的文件路径
- size: Option<i64> — 内容大小（字节）
- mime_type: Option<String> — MIME 类型
- created_at: NaiveDateTime — 创建时间
- updated_at: NaiveDateTime — 更新时间

### 内容视图模型 (RoomContentView)

- id: i64 — 内容 ID
- content_type: ContentType — 内容类型
- file_name: Option<String> — 文件名（从路径提取）
- url: Option<String> — URL 链接
- size: Option<i64> — 内容大小
- mime_type: Option<String> — MIME 类型
- created_at: NaiveDateTime — 创建时间
- updated_at: NaiveDateTime — 更新时间

### 内容类型枚举 (ContentType)

```rust
#[repr(i64)]
pub enum ContentType {
    Text = 0,    // 纯文本内容
    Image = 1,   // 图片文件
    File = 2,    // 通用文件
    Url = 3,     // URL 链接
}
```

> 数据库表：`room_contents`（迁移文件：`crates/board/migrations/002_create_room_contents_table.sql`）

## 3. 不变式 & 验证逻辑

### 业务规则

- 下载前必须获得有效的房间 JWT 令牌，且令牌具有查看权限
- 内容必须属于请求的房间（防止跨房间访问）
- 文件必须存在于磁盘上，否则返回 404 错误
- 房间状态必须为 Open 且未过期
- 不同内容类型有不同的处理逻辑：
  - File/Image: 从磁盘流式读取文件
  - Text: 直接返回文本内容
  - Url: 返回 URL 信息（不重定向）

### 验证逻辑

- 验证 JWT 令牌的有效性和权限
- 检查内容 ID 的存在性和所有权
- 验证文件路径的合法性和文件存在性
- 设置正确的 HTTP 响应头（Content-Type, Content-Disposition）

## 4. 持久化 & 索引

### 数据库表结构

```sql
CREATE TABLE IF NOT EXISTS room_contents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    content_type INTEGER NOT NULL, -- 0: text, 1: image, 2: file, 3: url
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

### 索引设计

- 主键索引：`id` 字段的自动索引
- 外键索引：`room_id` 字段用于优化按房间查询
- 复合索引：建议添加 `(room_id, content_type)` 优化类型过滤

### 文件存储

- 存储路径：`storage/rooms/{room_slug}/{uuid}_{filename}`
- 文件访问：直接从文件系统读取，使用流式传输

## 5. API/Handlers

### 获取房间内容列表

- **GET** `/api/v1/rooms/{name}/contents`
- 请求参数：房间名称、token
- 响应：房间内所有内容的列表
- 错误码：401（令牌无效）、403（权限不足）、404（房间不存在）

### 下载单个内容

- **GET** `/api/v1/rooms/{name}/contents/{content_id}`
- 请求参数：房间名称、内容 ID、token
- 响应：文件流或内容数据
- 错误码：401（令牌无效）、403（权限不足）、404（内容不存在）

### 请求示例

```bash
# 获取内容列表
GET /api/v1/rooms/myroom/contents?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...

# 下载文件
GET /api/v1/rooms/myroom/contents/123?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### 响应示例

```json
// 内容列表响应
[
  {
    "id": 123,
    "content_type": "File",
    "file_name": "document.pdf",
    "url": null,
    "size": 1024000,
    "mime_type": "application/pdf",
    "created_at": "2023-12-01T10:00:00",
    "updated_at": "2023-12-01T10:00:00"
  }
]

// 文件下载响应（HTTP 流）
Content-Type: application/pdf
Content-Disposition: attachment; filename="document.pdf"
[文件二进制数据...]
```

## 6. JWT 与权限

### 权限验证

- 使用 `verify_room_token` 函数验证 JWT 令牌
- 检查令牌中的 `permission` 字段是否包含查看权限 (`can_view()`)
- 验证令牌的 `room_id` 与目标房间匹配
- 确保令牌未被撤销且未过期

### 权限检查流程

```rust
ensure_permission(
    &verified.claims,
    verified.room.permission.can_view(),
    ContentPermission::View,
)?;
```

### 权限级别

- VIEW_ONLY: 可以查看和下载内容
- EDITABLE: 包含 VIEW 权限，可以上传和编辑
- DELETE: 包含以上权限，可以删除内容

## 7. 关键代码片段

### 获取内容列表 (crates/board/src/handlers/content.rs:126)

```rust
pub async fn list_contents(
    AxumPath(name): AxumPath<String>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<Vec<RoomContentView>> {
    // 验证房间名称
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    // 验证令牌和权限
    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    let room_id = room_id_or_error(&verified.claims)?;

    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    // 查询房间内容
    let repository = SqliteRoomContentRepository::new(app_state.db_pool.clone());
    let contents = repository.list_by_room(room_id).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to list contents: {e}"))
    })?;

    Ok(Json(contents.into_iter().map(RoomContentView::from).collect()))
}
```

### 下载文件内容 (crates/board/src/handlers/content.rs:637)

```rust
pub async fn download_content(
    AxumPath((name, content_id)): AxumPath<(String, i64)>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Response, HttpResponse> {
    // 验证令牌和权限
    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_view(),
        ContentPermission::View,
    )?;

    let room_id = room_id_or_error(&verified.claims)?;

    // 查询内容记录
    let repository = SqliteRoomContentRepository::new(app_state.db_pool.clone());
    let content = repository.find_by_id(content_id).await?
        .ok_or_else(|| HttpResponse::NotFound().message("Content not found"))?;

    // 验证内容属于该房间
    if content.room_id != room_id {
        return Err(HttpResponse::Forbidden().message("Content not in room"));
    }

    // 获取文件路径
    let path = content.path
        .ok_or_else(|| HttpResponse::NotFound().message("Content not stored on disk"))?;

    // 打开文件并创建流
    let file = fs::File::open(&path).await
        .map_err(|_| HttpResponse::NotFound().message("File missing on disk"))?;

    let file_name = Path::new(&path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "download.bin".to_string());

    // 创建流式响应
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let mut response = Response::new(body);

    // 设置响应头
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
        .map_err(|_| HttpResponse::InternalServerError().message("Failed to build response headers"))?;
    response.headers_mut().insert(CONTENT_DISPOSITION, disposition);

    if let Some(mime) = content.mime_type
        && let Ok(value) = HeaderValue::from_str(&mime)
    {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }

    Ok(response)
}
```

### 权限验证函数 (crates/board/src/handlers/content.rs:698)

```rust
fn ensure_permission(
    claims: &RoomTokenClaims,
    room_allows: bool,
    action: ContentPermission,
) -> Result<(), HttpResponse> {
    if !room_allows {
        return Err(HttpResponse::Forbidden().message("Permission denied by room"));
    }
    let permission = claims.as_permission();
    let token_allows = match action {
        ContentPermission::View => permission.can_view(),
        ContentPermission::Edit => permission.can_edit(),
        ContentPermission::Delete => permission.can_delete(),
    };
    if !token_allows {
        return Err(HttpResponse::Forbidden().message("Permission denied by token"));
    }
    Ok(())
}
```

## 8. 测试要点

### 单元测试建议

- 测试权限验证逻辑（不同权限级别的访问控制）
- 测试内容类型处理逻辑
- 测试文件路径验证和安全检查
- 测试 HTTP 响应头设置
- 测试错误处理和边界条件

### 集成测试建议

- 完整的下载流程：获取列表 → 下载文件
- 跨房间访问控制测试
- 大文件下载性能测试
- 并发下载场景测试
- 文件不存在的情况处理

### 边界条件测试

- 令牌过期的情况
- 文件被删除的情况
- 房间被关闭的情况
- 磁盘文件损坏的情况
- 网络中断恢复测试

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

- **访问日志记录**：当前缺乏详细的下载访问日志，建议添加访问记录用于审计
- **下载速度限制**：缺乏单个用户或房间的下载速率限制，可能被滥用

### P1 优先级

- **断点续传支持**：大文件下载失败后需要重新开始，建议支持 HTTP Range 请求
- **内容缓存机制**：频繁下载的文件缺乏缓存，建议添加内存或磁盘缓存

### P2 优先级

- **下载统计功能**：缺乏下载次数统计和热门内容分析
- **内容预览功能**：对于图片和文本，建议提供缩略图或预览功能

## 10. 关联文档 / 代码位置

### 源码路径

- 处理器实现：`crates/board/src/handlers/content.rs:126-696`
- 路由定义：`crates/board/src/route/room.rs:27-40`
- 数据模型：`crates/board/src/models/room/content.rs`
- 权限验证：`crates/board/src/handlers/content.rs:698-723`

### 数据库相关

- 迁移文件：`crates/board/migrations/002_create_room_contents_table.sql`
- 存储触发器：自动更新 `updated_at` 字段

### 测试文件

- 集成测试：`crates/board/tests/api_integration_tests.rs`
- 模型测试：`crates/board/src/models/room/content.rs:104-126`

### 相关文档

- [房间模型文档](model-room.md)
- [权限模型文档](model-permissions.md)
- [上传处理器文档](handler-upload.md)
- [令牌处理器文档](handler-token.md)
