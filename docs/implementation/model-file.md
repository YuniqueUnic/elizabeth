# File

## 1. 简介

File 模型（在代码中称为 `RoomContent`）是 Elizabeth
系统的文件存储和内容管理核心，负责管理房间内的各种类型内容。系统支持多种内容类型：文本、图片、文件和
URL，每种类型都有不同的存储和处理方式。File
模型实现了文件上传预留机制，确保大文件上传的可靠性和容量控制的准确性。主要交互方包括内容处理器（`crates/board/src/handlers/content.rs`）、房间模型和上传预留系统。

## 2. 数据模型（字段 & 类型 & 解释）

**RoomContent 结构体**（`crates/board/src/models/room/content.rs:19`）：

```rust
pub struct RoomContent {
    pub id: Option<i64>,              // 主键，数据库记录 ID
    pub room_id: i64,                 // 关联的房间 ID
    pub content_type: ContentType,    // 内容类型枚举
    pub text: Option<String>,         // 文本内容（Text 类型）
    pub url: Option<String>,          // URL 内容（Url 类型）
    pub path: Option<String>,         // 文件存储路径（File/Image 类型）
    pub size: Option<i64>,           // 内容大小（字节）
    pub mime_type: Option<String>,    // MIME 类型
    pub created_at: NaiveDateTime,    // 创建时间
    pub updated_at: NaiveDateTime,    // 更新时间
}
```

**ContentType 枚举**（`crates/board/src/models/room/content.rs:10`）：

```rust
#[repr(i64)]
pub enum ContentType {
    Text = 0,    // 纯文本内容
    Image = 1,   // 图片文件
    File = 2,    // 通用文件
    Url = 3,     // URL 链接
}
```

**UploadFileDescriptor 结构体**（用于上传预留）：

```rust
pub struct UploadFileDescriptor {
    pub name: String,           // 文件名
    pub size: i64,            // 文件大小
    pub mime: Option<String>,  // MIME 类型
}
```

**数据库映射**：对应 `crates/board/migrations/001_initial_schema.sql` 中的
`room_contents` 表。

## 3. 不变式 & 验证逻辑（业务规则）

- **内容类型一致性**：不同内容类型必须使用对应的存储字段（Text 用 `text`，File
  用 `path`，Url 用 `url`）
- **容量限制**：房间内所有内容的总大小不能超过房间的 `max_size` 限制
- **文件名安全**：上传的文件名经过安全化处理，防止路径遍历攻击
- **MIME 类型检测**：文件上传时自动检测 MIME 类型，支持后续的内容类型判断
- **上传预留机制**：大文件上传前必须先预留空间，确保容量不会超限
- **唯一性约束**：同一房间内可以有同名文件，但存储时会使用 UUID 前缀避免冲突

## 4. 持久化 & 索引（实现细节）

**数据库表结构**：

```sql
CREATE TABLE IF NOT EXISTS room_contents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    content_type INTEGER NOT NULL,  -- 0: text, 1: image, 2: file, 3: url
    text TEXT,                     -- 文本内容
    url TEXT,                      -- URL 内容
    path TEXT,                     -- 文件存储路径
    size INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT,                -- MIME 类型
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

**上传预留表**（`crates/board/migrations/001_initial_schema.sql`）：

```sql
CREATE TABLE IF NOT EXISTS room_upload_reservations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    token_jti TEXT NOT NULL,        -- 关联的 JWT JTI
    file_manifest TEXT NOT NULL,    -- JSON 格式的文件清单
    reserved_size INTEGER NOT NULL, -- 预留的总大小
    reserved_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,   -- 预留过期时间
    consumed_at DATETIME,           -- 实际消费时间
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

**索引和约束**：

- 主键：`id`（自增）
- 外键约束：`room_id` 关联到 `rooms.id`，级联删除
- 性能索引：预留表的 `room_id`、`token_jti`、`expires_at` 字段
- 自动触发器：更新时自动设置 `updated_at`

## 5. API/Handlers（对外行为）

**核心端点列表**：

- `GET /api/v1/rooms/{name}/contents` - 获取房间内容列表
  - 查询参数：`token: String`
  - 输出：`Vec<RoomContentView>`
  - 要求：预览权限
  - 实际路由：`crates/board/src/route/room.rs:25`
  - 处理函数：`crates/board/src/handlers/content.rs:list_contents`

- `POST /api/v1/rooms/{name}/contents/prepare` - 预留上传空间
  - 查询参数：`token: String`
  - 输入：`UploadPreparationRequest { files: Vec<UploadFileDescriptor> }`
  - 输出：`UploadPreparationResponse { reservation_id: i64, reserved_size: i64, ... }`
  - 要求：编辑权限
  - 实际路由：`crates/board/src/route/room.rs:21`
  - 处理函数：`crates/board/src/handlers/content.rs:prepare_upload`

- `POST /api/v1/rooms/{name}/contents` - 上传文件内容
  - 查询参数：`token: String, reservation_id: i64`
  - 输入：`multipart/form-data` 文件数据
  - 输出：`UploadContentResponse { uploaded: Vec<RoomContentView>, current_size: i64 }`
  - 要求：编辑权限
  - 实际路由：`crates/board/src/route/room.rs:23`
  - 处理函数：`crates/board/src/handlers/content.rs:upload_contents`

- `DELETE /api/v1/rooms/{name}/contents` - 删除房间内容
  - 查询参数：`token: String`
  - 输入：`DeleteContentRequest { ids: Vec<i64> }`
  - 输出：`DeleteContentResponse { deleted: Vec<i64>, freed_size: i64, current_size: i64 }`
  - 要求：删除权限
  - 实际路由：`crates/board/src/route/room.rs:26`
  - 处理函数：`crates/board/src/handlers/content.rs:delete_contents`

- `GET /api/v1/rooms/{name}/contents/{content_id}` - 下载文件内容
  - 查询参数：`token: String`
  - 输出：文件流响应
  - 要求：预览权限
  - 实际路由：`crates/board/src/route/room.rs:29`
  - 处理函数：`crates/board/src/handlers/content.rs:download_content`

**数据结构定义**（基于实际代码实现）：

```rust
// 上传文件描述符
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadFileDescriptor {
    pub name: String,           // 文件名
    pub size: i64,            // 文件大小
    pub mime: Option<String>,  // MIME 类型
}

// 上传准备请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadPreparationRequest {
    pub files: Vec<UploadFileDescriptor>,
}

// 上传准备响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadPreparationResponse {
    pub reservation_id: i64,
    pub reserved_size: i64,
    pub expires_at: String,  // ISO 8601 格式
    pub current_size: i64,
    pub remaining_size: i64,
    pub max_size: i64,
}

// 上传内容响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadContentResponse {
    pub uploaded: Vec<RoomContentView>,
    pub current_size: i64,
}

// 删除内容请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteContentRequest {
    pub ids: Vec<i64>,
}

// 删除内容响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteContentResponse {
    pub deleted: Vec<i64>,
    pub freed_size: i64,
    pub current_size: i64,
}

// 房间内容视图（用于 API 响应）
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoomContentView {
    pub id: i64,
    pub room_id: i64,
    pub content_type: ContentType,
    pub text: Option<String>,
    pub url: Option<String>,
    pub path: Option<String>,
    pub size: Option<i64>,
    pub mime_type: Option<String>,
    pub created_at: String,  // ISO 8601 格式
    pub updated_at: String,  // ISO 8601 格式
}
```

**请求/响应示例**：

```bash
# 1. 预留上传空间
curl -X POST "http://localhost:8080/api/v1/rooms/myroom/contents/prepare?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "files": [
      {
        "name": "document.pdf",
        "size": 1024000,
        "mime": "application/pdf"
      }
    ]
  }'

# 响应
{
  "reservation_id": 123,
  "reserved_size": 1024000,
  "expires_at": "2024-01-01T00:10:00",
  "current_size": 512000,
  "remaining_size": 9502720,
  "max_size": 10485760
}

# 2. 上传文件
curl -X POST "http://localhost:8080/api/v1/rooms/myroom/contents?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...&reservation_id=123" \
  -F "file=@document.pdf"

# 响应
{
  "uploaded": [
    {
      "id": 456,
      "room_id": 789,
      "content_type": "file",
      "text": null,
      "url": null,
      "path": "storage/rooms/789/uuid-document.pdf",
      "size": 1024000,
      "mime_type": "application/pdf",
      "created_at": "2024-01-01T00:05:00",
      "updated_at": "2024-01-01T00:05:00"
    }
  ],
  "current_size": 1536000
}

# 3. 获取房间内容列表
curl -X GET "http://localhost:8080/api/v1/rooms/myroom/contents?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# 响应
[
  {
    "id": 456,
    "room_id": 789,
    "content_type": "file",
    "text": null,
    "url": null,
    "path": "storage/rooms/789/uuid-document.pdf",
    "size": 1024000,
    "mime_type": "application/pdf",
    "created_at": "2024-01-01T00:05:00",
    "updated_at": "2024-01-01T00:05:00"
  },
  {
    "id": 457,
    "room_id": 789,
    "content_type": "text",
    "text": "Hello, World!",
    "url": null,
    "path": null,
    "size": 13,
    "mime_type": "text/plain",
    "created_at": "2024-01-01T00:06:00",
    "updated_at": "2024-01-01T00:06:00"
  }
]

# 4. 下载文件
curl -X GET "http://localhost:8080/api/v1/rooms/myroom/contents/456?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -o downloaded_document.pdf

# 5. 删除内容
curl -X DELETE "http://localhost:8080/api/v1/rooms/myroom/contents?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "ids": [456]
  }'

# 响应
{
  "deleted": [456],
  "freed_size": 1024000,
  "current_size": 13
}
```

**错误响应示例**：

```json
// 容量超限错误
{
  "error": "Room size limit exceeded",
  "code": 413,
  "message": "The uploaded file would exceed the room's maximum size limit"
}

// 权限不足错误
{
  "error": "Permission denied",
  "code": 403,
  "message": "You don't have EDITABLE permission for this room"
}

// 预留过期错误
{
  "error": "Reservation expired",
  "code": 410,
  "message": "The upload reservation has expired"
}
```

## 6. JWT 与权限（如何生成/校验）

File 模型本身不直接处理 JWT，但所有操作都需要 JWT 验证：

- **权限检查**：每个操作前都验证 JWT 中的对应权限
  - 查看内容：需要 `VIEW_ONLY` 权限
  - 上传内容：需要 `EDITABLE` 权限
  - 删除内容：需要 `DELETE` 权限

- **容量验证**：JWT 中包含房间的 `max_size` 信息，用于容量控制
- **房间关联**：JWT 中的 `room_id` 确保操作只能发生在对应房间内

## 7. 关键代码片段（无需粘全部，提供入口/关键函数）

**RoomContent 构建器模式**（`crates/board/src/models/room/content.rs:34`）：

```rust
#[bon::bon]
impl RoomContent {
    #[builder]
    pub fn builder(
        id: Option<i64>,
        room_id: i64,
        content_type: ContentType,
        now: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            room_id,
            content_type,
            created_at: now,
            updated_at: now,
            text: None,
            url: None,
            path: None,
            size: None,
            mime_type: None,
        }
    }
}
```

**文本内容设置**（`crates/board/src/models/room/content.rs:56`）：

```rust
pub fn set_text(&mut self, text: String) {
    let size = text.len() as i64; // 当前使用字符串长度作为大小计算
                                    // ⚠️ 精度警告：未考虑实际编码开销，对于非 ASCII 字符可能不准确
    self.text = Some(text);
    self.updated_at = Utc::now().naive_utc();
    self.mime_type = Some("text/plain".to_string());
    self.size = Some(size);
}
```

**文件路径设置**（`crates/board/src/models/room/content.rs:64`）：

```rust
pub fn set_path(
    &mut self,
    path: String,
    content_type: ContentType,
    size: i64,
    mime_type: String,
) {
    self.path = Some(path);
    self.content_type = content_type;
    self.updated_at = Utc::now().naive_utc();
    self.mime_type = Some(mime_type);
    self.size = Some(size);
}
```

**URL 内容设置**（`crates/board/src/models/room/content.rs:78`）：

```rust
pub fn set_url(&mut self, url: String, mime_type: Option<String>) {
    let size = url.len() as i64; // 当前使用 URL 字符串长度作为大小计算
                                 // ⚠️ 精度警告：未考虑实际编码开销，对于非 ASCII 字符可能不准确
    self.url = Some(url);
    self.content_type = ContentType::Url;
    self.updated_at = Utc::now().naive_utc();
    self.mime_type = mime_type;
    self.size = Some(size);
}
```

**文件大小计算方法说明**：

当前系统中不同内容类型的大小计算方法如下：

1. **文本内容 (Text)**：使用 `text.len()` 计算字符串长度（字节数）
2. **文件内容 (File/Image)**：使用传入的实际文件大小（字节）
3. **URL 内容 (Url)**：使用 `url.len()` 计算 URL 字符串长度（字节数）

**计算精度说明**：

- 文本和 URL 内容的大小计算基于字符串长度，未考虑实际编码开销
- 文件内容的大小基于实际文件大小，较为准确
- 所有大小值都以字节为单位存储为 `i64` 类型

**上传预留逻辑**（`crates/board/src/handlers/content.rs:217`）：

```rust
let (reservation, updated_room) = reservation_repo
    .reserve_upload(
        &verified.room,
        &verified.claims.jti,
        &manifest_json,
        total_size,
        ttl,
    )
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.to_lowercase().contains("limit exceeded") {
            HttpResponse::PayloadTooLarge().message("Room size limit exceeded")
        } else {
            HttpResponse::InternalServerError().message(format!("Reserve upload failed: {msg}"))
        }
    })?;
```

## 8. 测试要点（单元/集成测试建议）

- **基础功能测试**：
  - 不同内容类型的创建、读取、更新、删除
  - 文件上传的完整流程（预留 → 上传 → 确认）
  - MIME 类型检测的正确性

- **容量控制测试**：
  - 房间容量限制的 enforcement
  - 上传预留机制的正确性
  - 容量超限时的错误处理

- **安全性测试**：
  - 文件名安全化处理
  - 路径遍历攻击防护
  - 恶意文件类型检测

- **并发测试**：
  - 多用户同时上传文件
  - 上传预留的并发控制
  - 容量计算的原子性

- **集成测试**：
  - 完整的文件生命周期：上传 → 查看 → 下载 → 删除
  - 不同权限下的操作验证
  - 房间删除时文件的清理

## 9. 已知问题 / TODO / 改进建议

**P0 优先级**：

- **容量计算精度**：文本和 URL 内容大小计算存在精度问题
  - **问题描述**：文本内容使用 `text.len()` 计算大小，URL 内容使用 `url.len()`
    计算大小，未考虑实际编码开销
  - **精度影响**：对于包含非 ASCII
    字符（如中文、日文、表情符号等）的内容，实际存储大小可能远大于计算值
  - **潜在风险**：可能导致房间容量限制不准确，允许上传超过限制的内容或过早拒绝合法内容
  - **建议解决方案**：使用 `text.as_bytes().len()` 计算实际字节数，或使用
    `text.encode_utf8().len()` 确保编码一致性
  - **实施建议**：在下一个版本中修复容量计算逻辑，并提供数据迁移脚本处理现有数据
- **文件存储安全**：缺少文件内容的安全扫描和病毒检测

**P1 优先级**：

- **大文件分片上传**：当前不支持大文件的分片上传和断点续传
- **文件版本管理**：不支持文件的版本控制和历史记录
- **存储路径优化**：缺少基于日期或类型的存储路径分层

**P2 优先级**：

- **内容压缩**：不支持文本内容的自动压缩存储
- **文件转换服务**：不支持图片格式转换或文档预览生成
- **CDN 集成**：不支持 CDN 集成和分布式存储

## 10. 关联文档 / 代码位置

**源码路径**：

- 内容模型：`crates/board/src/models/room/content.rs`
- 上传预留：`crates/board/src/models/room/upload_reservation.rs`
- 内容处理器：`crates/board/src/handlers/content.rs`
- 内容仓储：`crates/board/src/repository/room_content_repostitory.rs`
- 上传预留仓储：`crates/board/src/repository/room_upload_reservation_repository.rs`

**数据库迁移**：

- 内容表和上传预留表：`crates/board/migrations/001_initial_schema.sql`

**测试文件路径**：

- 单元测试：`crates/board/src/models/room/content.rs:104` 中的 `#[cfg(test)]` 块
- 集成测试：`crates/board/tests/api_integration_tests.rs`

**关联文档**：

- [model-room.md](./model-room.md) - 房间模型详细说明
- [model-permissions.md](./model-permissions.md) - 权限系统详细说明
- [model-session-jwt.md](./model-session-jwt.md) - JWT 令牌机制

**存储配置**：

- 默认存储根目录：`storage/rooms/`
- 文件名安全化：使用 `sanitize_filename` crate
- MIME 类型检测：使用 `mime_guess` crate
