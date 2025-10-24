# 分块上传进度跟踪 (Chunked Upload Progress)

## 1. 简介

分块上传进度跟踪是 Elizabeth
系统中支持大文件上传的核心功能。该功能允许用户上传大文件时实时查看上传进度，支持断点续传，并在网络中断后恢复上传。系统通过数据库视图和专门的
API 端点提供进度查询功能，确保用户能够准确了解上传状态。

## 2. 数据模型

### 分块上传状态视图 (v_chunked_upload_status)

```sql
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

### 上传状态枚举

- `pending`: 等待上传
- `uploading`: 正在上传
- `completed`: 上传完成
- `failed`: 上传失败
- `expired`: 预留过期

## 3. API 端点

### 查询上传状态

- **GET** `/api/v1/rooms/{name}/uploads/chunks/status`
- 请求参数：房间名称、token、reservation_id
- 响应：上传进度信息

### 请求示例

```bash
GET /api/v1/rooms/myroom/uploads/chunks/status?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...&reservation_id=123
```

### 响应示例

```json
{
  "reservation_id": 123,
  "upload_progress": 60.0,
  "uploaded_chunks": 6,
  "total_chunks": 10,
  "upload_status": "uploading",
  "chunk_size": 1048576,
  "file_size": 10485760,
  "expires_at": "2023-12-01T10:00:00",
  "verified_chunks": 6
}
```

## 4. 实现细节

### 进度计算逻辑

```rust
// 计算上传进度
let progress = if total_chunks > 0 {
    (uploaded_chunks as f64 / total_chunks as f64) * 100.0
} else {
    0.0
};
```

### 状态更新机制

1. **分块上传时**：每成功上传一个分块，更新 `uploaded_chunks` 计数
2. **完成时**：将状态更新为 `completed`
3. **失败时**：将状态更新为 `failed`
4. **过期时**：通过定时任务将过期预留状态更新为 `expired`

### 数据库查询优化

```sql
-- 高效的进度查询
SELECT
    rur.id,
    rur.total_chunks,
    rur.uploaded_chunks,
    COUNT(rcu.id) as uploaded_count,
    rur.upload_status
FROM room_upload_reservations rur
LEFT JOIN room_chunk_uploads rcu ON rur.id = rcu.reservation_id
    AND rcu.upload_status = 'uploaded'
WHERE rur.id = ? AND rur.chunked_upload = TRUE
GROUP BY rur.id;
```

## 5. 错误处理

### 常见错误情况

1. **预留不存在**：返回 404 错误
2. **预留过期**：返回 400 错误，提示重新开始上传
3. **权限不足**：返回 403 错误
4. **分块不完整**：返回 400 错误，提示缺失的分块

### 错误响应示例

```json
{
  "error": "Reservation not found",
  "code": "RESERVATION_NOT_FOUND",
  "message": "The specified reservation does not exist or has expired"
}
```

## 6. 性能优化

### 数据库优化

1. **索引设计**：
   - `idx_room_chunk_uploads_reservation_status`: 复合索引优化状态查询
   - `idx_room_upload_reservations_chunked_upload`: 优化分块上传查询

2. **查询优化**：
   - 使用视图减少复杂查询的计算开销
   - 避免 N+1 查询问题

### 缓存策略

1. **内存缓存**：短期缓存频繁查询的进度信息
2. **过期策略**：缓存时间不超过 5 秒，确保实时性

## 7. 监控指标

### 关键指标

1. **上传成功率**：完成的分块上传占总数的比例
2. **平均上传时间**：从开始到完成上传的平均时间
3. **失败率**：失败的上传占总数的比例
4. **并发上传数**：同时进行的分块上传数量

### 监控查询

```sql
-- 上传成功率统计
SELECT
    upload_status,
    COUNT(*) as count,
    COUNT(*) * 100.0 / SUM(COUNT(*)) OVER() as percentage
FROM room_upload_reservations
WHERE chunked_upload = TRUE
  AND created_at >= datetime('now', '-24 hours')
GROUP BY upload_status;
```

## 8. 测试要点

### 单元测试

- 进度计算准确性测试
- 状态转换逻辑测试
- 边界条件处理测试

### 集成测试

- 完整的分块上传流程测试
- 并发上传进度查询测试
- 网络中断恢复测试

### 性能测试

- 大文件上传性能测试
- 高并发进度查询测试
- 长时间上传稳定性测试

## 9. 已知限制

1. **并发限制**：单个房间最多支持 10 个并发分块上传
2. **文件大小限制**：单文件最大支持 10GB
3. **分块大小限制**：分块大小必须在 1MB-100MB 之间
4. **预留过期时间**：分块上传预留有效期为 1 小时

## 10. 未来改进

### 计划功能

1. **WebSocket 实时推送**：实时推送上传进度变化
2. **断点续传优化**：更智能的断点续传策略
3. **上传速度限制**：防止带宽滥用
4. **分块并行上传**：支持多分块并行上传

### 技术改进

1. **分布式存储**：支持多节点分块存储
2. **压缩传输**：支持分块压缩传输
3. **加密传输**：端到端分块加密

## 11. 关联文档

- [分块上传设计文档](chunked-upload-design.md)
- [分块上传 API 文档](chunked-upload-api.md)
- [文件合并 API 文档](chunked-upload-file-merge-api.md)
- [上传处理器文档](handler-upload.md)

## 12. 代码位置

- 分块上传处理器：`crates/board/src/handlers/chunked_upload.rs`
- 分块上传模型：`crates/board/src/models/room/chunk_upload.rs`
- 分块上传仓库：`crates/board/src/repository/room_chunk_upload_repository.rs`
- 数据库迁移：`crates/board/migrations/003_chunked_upload.sql`

---

**文档最后更新时间**：2025-10-23 **文档作者**：Elizabeth 开发团队
**文档版本**：v1.0.0
