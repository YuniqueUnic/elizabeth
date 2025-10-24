# 分块上传 API 实现

## 概述

本文档描述了 Elizabeth 项目中实现的分块上传预留 API 端点。该 API
端点允许客户端在执行分块上传前预留上传空间，提供更好的上传管理和并发控制。

## API 端点

### 预留分块上传空间

**端点**: `POST /api/v1/rooms/{name}/uploads/chunks/prepare`

**描述**: 为指定房间预留分块上传空间，生成上传令牌和预留 ID。

**请求参数**:

- `name` (路径参数): 房间名称

**请求体**:

```json
{
  "files": [
    {
      "name": "example.pdf",
      "size": 10485760,
      "mime": "application/pdf",
      "chunk_size": 1048576,
      "file_hash": "sha256:abc123..."
    }
  ]
}
```

**响应**:

```json
{
  "reservation_id": "uuid-v4",
  "upload_token": "uuid-v4",
  "expires_at": "2023-12-01T12:00:00Z",
  "files": [
    {
      "name": "example.pdf",
      "size": 10485760,
      "mime": "application/pdf",
      "chunk_size": 1048576,
      "total_chunks": 10,
      "file_hash": "sha256:abc123..."
    }
  ]
}
```

## 实现细节

### 数据模型

#### ChunkedUploadPreparationRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkedUploadPreparationRequest {
    pub files: Vec<UploadFileDescriptor>,
}
```

#### UploadFileDescriptor

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UploadFileDescriptor {
    pub name: String,
    pub size: i64,
    pub mime: Option<String>,
    pub chunk_size: Option<i32>,
    pub file_hash: Option<String>,
}
```

#### ChunkedUploadPreparationResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkedUploadPreparationResponse {
    pub reservation_id: String,
    pub upload_token: String,
    pub expires_at: NaiveDateTime,
    pub files: Vec<ReservedFileInfo>,
}
```

#### ReservedFileInfo

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReservedFileInfo {
    pub name: String,
    pub size: i64,
    pub mime: Option<String>,
    pub chunk_size: i64,
    pub total_chunks: i64,
    pub file_hash: Option<String>,
}
```

#### UploadStatusQuery

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UploadStatusQuery {
    pub upload_token: Option<String>,
    pub reservation_id: Option<String>,
}
```

#### ChunkStatusInfo

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkStatusInfo {
    pub chunk_index: i64,
    pub chunk_size: i64,
    pub chunk_hash: Option<String>,
    pub upload_status: ChunkStatus,
    pub uploaded_at: Option<NaiveDateTime>,
}
```

#### UploadStatusResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UploadStatusResponse {
    pub reservation_id: String,
    pub upload_token: String,
    pub upload_status: UploadStatus,
    pub total_chunks: i64,
    pub uploaded_chunks: i64,
    pub progress_percentage: f64,
    pub expires_at: NaiveDateTime,
    pub chunk_details: Vec<ChunkStatusInfo>,
    pub reserved_size: i64,
    pub uploaded_size: i64,
    pub is_expired: bool,
    pub remaining_seconds: Option<i64>,
}
```

### 处理流程

1. **验证请求参数**:
   - 检查房间名称是否有效
   - 验证文件列表不为空
   - 验证每个文件的基本信息（名称、大小等）

2. **检查房间状态**:
   - 验证房间是否存在
   - 检查房间是否允许上传
   - 验证房间是否有足够的空间

3. **计算预留参数**:
   - 计算总预留大小
   - 为每个文件计算分块信息
   - 生成预留 ID 和上传令牌

4. **创建预留记录**:
   - 使用`IRoomUploadReservationRepository`接口创建预留记录
   - 设置预留过期时间
   - 标记为分块上传类型

5. **构建响应**:
   - 返回预留 ID 和上传令牌
   - 返回每个文件的详细分块信息

### 错误处理

API 提供以下错误响应：

- `400 Bad Request`: 请求参数无效
- `403 Forbidden`: 房间不允许上传或空间不足
- `404 Not Found`: 房间不存在
- `500 Internal Server Error`: 服务器内部错误

## 安全考虑

1. **权限验证**: API 会检查房间的上传权限
2. **空间限制**: API 会验证房间是否有足够的空间容纳请求的文件
3. **令牌管理**: 上传令牌具有有效期，过期后需要重新获取
4. **并发控制**: 预留机制防止多个上传同时占用相同空间

## 使用示例

### 预留上传空间

```bash
curl -X POST "http://localhost:8080/api/v1/rooms/my-room/uploads/chunks/prepare" \
  -H "Content-Type: application/json" \
  -d '{
    "files": [
      {
        "name": "large-file.zip",
        "size": 52428800,
        "mime": "application/zip",
        "chunk_size": 1048576
      }
    ]
  }'
```

### 查询上传状态（使用上传令牌）

```bash
curl -X GET "http://localhost:8080/api/v1/rooms/my-room/uploads/chunks/status?token=uuid-v4" \
  -H "Content-Type: application/json"
```

### 查询上传状态（使用预留 ID）

```bash
curl -X GET "http://localhost:8080/api/v1/rooms/my-room/uploads/chunks/status?reservation_id=uuid-v4" \
  -H "Content-Type: application/json"
```

### 查询上传状态

**端点**: `GET /api/v1/rooms/{name}/uploads/chunks/status`

**描述**: 查询分块上传的进度和状态信息。

**请求参数**:

- `name` (路径参数): 房间名称
- `token` (查询参数，可选): 上传令牌
- `reservation_id` (查询参数，可选): 预留 ID

**注意**: `token` 和 `reservation_id` 必须提供其中一个，但不能同时提供。

**响应**:

```json
{
  "reservation_id": "uuid-v4",
  "upload_token": "uuid-v4",
  "upload_status": "uploading",
  "total_chunks": 10,
  "uploaded_chunks": 6,
  "progress_percentage": 60.0,
  "expires_at": "2023-12-01T12:00:00Z",
  "chunk_details": [
    {
      "chunk_index": 0,
      "chunk_size": 1048576,
      "chunk_hash": "sha256:abc123...",
      "upload_status": "completed",
      "uploaded_at": "2023-12-01T10:30:00Z"
    },
    {
      "chunk_index": 1,
      "chunk_size": 1048576,
      "chunk_hash": "sha256:def456...",
      "upload_status": "completed",
      "uploaded_at": "2023-12-01T10:31:00Z"
    }
  ],
  "reserved_size": 10485760,
  "uploaded_size": 6291456,
  "is_expired": false,
  "remaining_seconds": 3600
}
```

**状态值说明**:

- `pending`: 等待上传（尚未开始）
- `uploading`: 正在上传中
- `completed`: 上传完成
- `expired`: 预留已过期

## 后续步骤

1. 使用返回的`upload_token`进行分块上传
2. 每个分块上传时需要包含`reservation_id`
3. 使用状态查询 API 监控上传进度
4. 所有分块上传完成后，系统会自动合并文件
5. 预留记录会在上传完成或过期后自动清理

## 技术实现

- **异步处理**: 使用 Rust 的`async/await`进行异步处理
- **错误处理**: 完整的错误处理链，包括数据库错误和业务逻辑错误
- **日志记录**: 关键操作都有日志记录
- **类型安全**: 使用 Rust 的类型系统确保 API 安全性
- **RESTful 设计**: 遵循 RESTful API 设计原则
- **并发处理**: 能够处理多个并发的预留请求

## 相关文件

- `crates/board/src/handlers/chunked_upload.rs`: 处理程序实现
- `crates/board/src/models/room/upload_reservation.rs`: 预留数据模型
- `crates/board/src/models/room/chunk_upload.rs`: 分块数据模型
- `crates/board/src/repository/room_upload_reservation_repository.rs`:
  预留数据访问层
- `crates/board/src/repository/room_chunk_upload_repository.rs`: 分块数据访问层
- `crates/board/src/route/room.rs`: 路由配置
