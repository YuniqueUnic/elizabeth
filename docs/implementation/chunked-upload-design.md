# 断点续传功能设计方案

## 1. 概述

本文档详细描述了 Elizabeth
项目中断点续传功能的设计方案，旨在解决大文件上传可靠性问题，支持分块上传、断点恢复和状态管理。

## 2. 问题分析

### 2.1 当前问题

- 文件上传缺少断点续传功能
- 大文件上传失败后需要重新开始
- 上传状态管理不足
- TTL 时间过短（10 秒）不适合大文件上传

### 2.2 需求分析

- 支持分块上传机制
- 实现断点恢复功能
- 提供上传状态跟踪
- 延长上传会话有效期
- 确保文件完整性验证

## 3. 技术架构设计

### 3.1 整体架构

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   客户端应用    │    │   API网关       │    │   上传服务      │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │分块上传逻辑 │ │◄──►│ │路由分发     │ │◄──►│ │预留管理     │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │状态跟踪     │ │    │ │权限验证     │ │    │ │分块处理     │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │断点恢复     │ │    │ │限流控制     │ │    │ │文件合并     │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   数据存储层    │
                    │                 │
                    │ ┌─────────────┐ │
                    │ │元数据数据库 │ │
                    │ └─────────────┘ │
                    │ ┌─────────────┐ │
                    │ │分块文件存储 │ │
                    │ └─────────────┘ │
                    │ ┌─────────────┐ │
                    │ │最终文件存储 │ │
                    │ └─────────────┘ │
                    └─────────────────┘
```

### 3.2 数据模型设计

#### 3.2.1 扩展 RoomUploadReservation 模型

```rust
// crates/board/src/models/room/upload_reservation.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomUploadReservation {
    pub id: i64,
    pub room_id: i64,
    pub token_jti: String,
    pub file_manifest: String,
    pub reserved_size: i64,
    pub reserved_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub consumed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    // 新增字段支持分块上传
    pub chunked_upload: bool,           // 是否为分块上传
    pub total_chunks: Option<i32>,      // 总分块数
    pub uploaded_chunks: Option<i32>,   // 已上传分块数
    pub file_hash: Option<String>,      // 文件完整哈希
    pub chunk_size: Option<i32>,        // 分块大小
    pub upload_status: UploadStatus,    // 上传状态
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum UploadStatus {
    Pending,       // 等待上传
    Uploading,     // 上传中
    Completed,     // 已完成
    Failed,        // 失败
    Expired,       // 已过期
}
```

#### 3.2.2 新增 RoomChunkUpload 模型

```rust
// crates/board/src/models/room/chunk_upload.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomChunkUpload {
    pub id: i64,
    pub reservation_id: i64,           // 关联的预留 ID
    pub chunk_index: i32,              // 分块索引（从 0 开始）
    pub chunk_size: i32,               // 分块大小
    pub chunk_hash: Option<String>,    // 分块哈希值
    pub upload_status: ChunkStatus,    // 分块状态
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum ChunkStatus {
    Pending,   // 等待上传
    Uploaded,  // 已上传
    Verified,  // 已验证
    Failed,    // 失败
}
```

### 3.3 数据库设计

#### 3.3.1 扩展 room_upload_reservations 表

```sql
-- 扩展现有表
ALTER TABLE room_upload_reservations ADD COLUMN chunked_upload BOOLEAN DEFAULT FALSE;
ALTER TABLE room_upload_reservations ADD COLUMN total_chunks INTEGER;
ALTER TABLE room_upload_reservations ADD COLUMN uploaded_chunks INTEGER DEFAULT 0;
ALTER TABLE room_upload_reservations ADD COLUMN file_hash TEXT;
ALTER TABLE room_upload_reservations ADD COLUMN chunk_size INTEGER;
ALTER TABLE room_upload_reservations ADD COLUMN upload_status TEXT DEFAULT 'pending';

-- 添加索引
CREATE INDEX idx_room_upload_reservations_chunked_upload ON room_upload_reservations(chunked_upload);
CREATE INDEX idx_room_upload_reservations_upload_status ON room_upload_reservations(upload_status);
```

#### 3.3.2 新增 room_chunk_uploads 表

```sql
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

-- 添加索引
CREATE INDEX idx_room_chunk_uploads_reservation_id ON room_chunk_uploads(reservation_id);
CREATE INDEX idx_room_chunk_uploads_status ON room_chunk_uploads(upload_status);
CREATE INDEX idx_room_chunk_uploads_chunk_index ON room_chunk_uploads(chunk_index);
```

## 4. API 设计

### 4.1 分块上传预留 API

```http
POST /api/v1/rooms/{name}/contents/prepare-chunked
Content-Type: application/json

{
  "files": [
    {
      "name": "large-file.zip",
      "size": 104857600,
      "mime": "application/zip",
      "chunk_size": 1048576,
      "file_hash": "sha256:abc123..."
    }
  ]
}

Response:
{
  "reservation_id": 123,
  "chunked_upload": true,
  "total_chunks": 100,
  "chunk_size": 1048576,
  "expires_at": "2023-12-01T10:00:00",
  "upload_status": "pending"
}
```

### 4.2 单个分块上传 API

```http
POST /api/v1/rooms/{name}/contents/chunk
Content-Type: multipart/form-data

Query Parameters:
- token: JWT令牌
- reservation_id: 预留ID
- chunk_index: 分块索引
- chunk_hash: 分块哈希值

Form Data:
- chunk: 分块文件数据

Response:
{
  "chunk_index": 42,
  "upload_status": "uploaded",
  "uploaded_chunks": 43,
  "total_chunks": 100,
  "upload_progress": 43.0
}
```

### 4.3 上传状态查询 API

```http
GET /api/v1/rooms/{name}/contents/chunk-status?token=xxx&reservation_id=123

Response:
{
  "reservation_id": 123,
  "upload_status": "uploading",
  "total_chunks": 100,
  "uploaded_chunks": 43,
  "upload_progress": 43.0,
  "chunk_status": [
    {"chunk_index": 0, "status": "verified"},
    {"chunk_index": 1, "status": "verified"},
    // ... 其他分块状态
  ]
}
```

### 4.4 文件合并完成 API

```http
POST /api/v1/rooms/{name}/contents/complete
Content-Type: application/json

{
  "reservation_id": 123,
  "final_hash": "sha256:abc123..."
}

Response:
{
  "reservation_id": 123,
  "upload_status": "completed",
  "file_info": {
    "name": "large-file.zip",
    "size": 104857600,
    "hash": "sha256:abc123...",
    "path": "storage/rooms/room123/uuid_large-file.zip"
  }
}
```

## 5. 存储策略

### 5.1 分块临时存储

```
storage/rooms/{room_slug}/.chunks/{reservation_id}/
├── chunk_000000
├── chunk_000001
├── chunk_000002
└── ...
```

### 5.2 最终文件存储

```
storage/rooms/{room_slug}/
├── uuid_filename.ext
└── uuid_other_file.ext
```

### 5.3 清理策略

- 成功合并后自动清理临时分块
- 过期预留的分块文件定期清理
- 失败上传的临时文件延迟清理

## 6. 配置参数

```rust
// crates/board/src/handlers/content.rs
pub const DEFAULT_CHUNK_SIZE: i64 = 1024 * 1024; // 1MB
pub const CHUNKED_UPLOAD_TTL_SECONDS: i64 = 24 * 60 * 60; // 24 小时
pub const MAX_CHUNKS_PER_FILE: i32 = 10000;
pub const CHUNK_UPLOAD_TIMEOUT_SECONDS: i64 = 30;
pub const CHUNK_CLEANUP_INTERVAL_SECONDS: i64 = 60 * 60; // 1 小时
```

## 7. 安全考虑

### 7.1 权限验证

- 使用现有 JWT 令牌验证机制
- 确保分块上传与预留匹配
- 验证房间编辑权限

### 7.2 完整性验证

- 每个分块的哈希验证
- 最终文件的完整性校验
- 防止部分文件篡改

### 7.3 资源保护

- 限制单文件最大分块数
- 限制并发分块上传数
- 磁盘空间使用监控

## 8. 错误处理

### 8.1 网络中断处理

- 分块上传失败自动重试
- 会话超时后可恢复
- 状态查询确认上传进度

### 8.2 磁盘空间不足

- 预检查磁盘可用空间
- 优雅拒绝新上传请求
- 清理过期临时文件

### 8.3 并发控制

- 防止同一文件重复上传
- 分块上传顺序控制
- 资源锁定机制

## 9. 性能优化

### 9.1 并发上传

- 支持多分块并行上传
- 异步分块处理
- 连接池优化

### 9.2 内存管理

- 流式文件处理
- 分块缓冲区复用
- 大文件内存映射

### 9.3 数据库优化

- 批量分块状态更新
- 索引优化查询
- 连接池配置

## 10. 监控指标

### 10.1 上传性能指标

- 分块上传成功率
- 平均上传速度
- 并发上传数

### 10.2 系统资源指标

- 磁盘空间使用
- 内存使用情况
- 数据库连接数

### 10.3 错误统计

- 网络中断次数
- 验证失败次数
- 超时错误统计

## 11. 实施计划

### 11.1 第一阶段：数据模型和基础设施（3-4 天）

- [ ] 扩展 RoomUploadReservation 模型
- [ ] 创建 RoomChunkUpload 模型
- [ ] 数据库迁移脚本
- [ ] 基础配置参数

### 11.2 第二阶段：核心 API 实现（4-5 天）

- [ ] 分块上传预留 API
- [ ] 分块上传 API
- [ ] 上传状态查询 API
- [ ] 文件合并完成 API

### 11.3 第三阶段：文件处理和存储（3-4 天）

- [ ] 分块存储逻辑
- [ ] 文件合并算法
- [ ] 完整性验证
- [ ] 临时文件清理

### 11.4 第四阶段：错误处理和优化（2-3 天）

- [ ] 网络中断恢复
- [ ] 超时处理
- [ ] 并发控制
- [ ] 性能优化

### 11.5 第五阶段：测试和文档（3-4 天）

- [ ] 单元测试
- [ ] 集成测试
- [ ] 文档更新
- [ ] 部署验证

**总计：15-20 天的开发周期**

## 12. 风险评估

### 12.1 技术风险

- **存储空间风险**：临时分块可能占用大量磁盘空间
- **性能风险**：大量并发分块上传可能影响系统性能
- **数据一致性风险**：部分上传失败可能导致数据不一致

### 12.2 缓解措施

- 实施严格的存储配额和清理机制
- 限制并发上传数量和速率
- 实现事务性操作和回滚机制
- 充分的测试覆盖和监控告警

## 13. 向后兼容性

### 13.1 兼容策略

- 保持现有上传 API 完全兼容
- 新功能通过新 API 端点提供
- 数据库迁移不破坏现有数据

### 13.2 迁移计划

- 渐进式功能发布
- 客户端适配器支持
- 充分的回滚准备

---

**文档版本**: v1.0 **创建日期**: 2025-10-23 **作者**: 系统架构师 **审核状态**:
待审核
