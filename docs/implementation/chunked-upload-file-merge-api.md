# 分块上传文件合并完成 API 端点实现

## 概述

本文档记录了 Elizabeth 项目中分块上传文件合并完成 API
端点的实现。该功能允许客户端在所有分块上传完成后，触发文件合并过程，将分块文件合并为完整文件。

## 实现日期

2025-10-23

## 功能描述

文件合并完成 API 端点处理以下功能：

- 接收文件合并完成请求（包含预留 ID 和最终文件哈希）
- 验证上传预留记录的有效性
- 检查所有分块是否已上传完成
- 验证文件完整性（哈希校验）
- 按顺序合并所有分块为完整文件
- 将合并后的文件移动到最终存储位置
- 更新预留记录状态为已完成
- 清理临时分块文件
- 返回合并完成的响应，包含文件信息

## API 端点

### 端点信息

- **路径**: `/api/v1/rooms/{name}/uploads/chunks/complete`
- **方法**: POST
- **描述**: 完成分块上传文件的合并

### 请求参数

#### 路径参数

- `name` (String, Path): 房间名称

#### 请求体

```json
{
  "reservation_id": 123,
  "final_hash": "sha256_hash_of_complete_file"
}
```

- `reservation_id` (i64): 预留记录 ID
- `final_hash` (String): 完整文件的 SHA256 哈希值

### 响应

#### 成功响应 (200)

```json
{
  "reservation_id": 123,
  "upload_status": "completed",
  "file_info": {
    "name": "example.pdf",
    "size": 1048576,
    "hash": "sha256_hash_of_complete_file",
    "path": "storage/rooms/456/example.pdf"
  }
}
```

#### 错误响应

- **400 Bad Request**: 请求参数错误、分块未全部上传、文件哈希验证失败
- **403 Forbidden**: 预留记录已过期或不属于指定房间
- **404 Not Found**: 房间不存在或预留记录不存在
- **409 Conflict**: 文件已完成合并或上传已失败
- **500 Internal Server Error**: 服务器内部错误

## 实现细节

### 数据模型

#### FileMergeRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FileMergeRequest {
    pub reservation_id: i64,
    pub final_hash: String,
}
```

#### FileMergeResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FileMergeResponse {
    pub reservation_id: i64,
    pub upload_status: String,
    pub file_info: MergedFileInfo,
}
```

#### MergedFileInfo

```rust
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MergedFileInfo {
    pub name: String,
    pub size: i64,
    pub hash: String,
    pub path: String,
}
```

### 核心处理逻辑

#### 1. 验证阶段

- 验证房间名称有效性
- 验证预留记录存在且属于指定房间
- 验证预留记录为分块上传类型
- 验证预留记录未过期
- 验证预留记录状态允许合并

#### 2. 分块检查阶段

- 查询所有关联的分块记录
- 验证分块数量与预期一致
- 验证所有分块状态为已上传

#### 3. 文件合并阶段

- 按分块索引排序分块记录
- 创建临时合并文件
- 按顺序读取并写入每个分块数据
- 验证合并后文件的哈希值

#### 4. 文件存储阶段

- 创建最终存储目录
- 从文件清单获取文件名
- 将合并文件移动到最终位置
- 更新预留记录状态为已完成
- 标记预留记录为已消费

#### 5. 清理阶段

- 清理临时分块文件和目录
- 记录清理失败的警告

### 错误处理

#### 验证错误

- 预留记录不存在或无效
- 分块未全部上传
- 文件哈希验证失败

#### 系统错误

- 文件系统操作失败
- 数据库操作失败
- 内存不足等系统资源错误

#### 错误恢复

- 合并失败时更新预留记录状态为失败
- 清理部分创建的临时文件
- 提供详细的错误信息

### 安全考虑

#### 权限验证

- 验证预留记录属于指定房间
- 验证用户有房间上传权限

#### 完整性验证

- 验证所有分块已上传
- 验证合并后文件的哈希值
- 防止部分文件替换攻击

#### 资源管理

- 限制合并操作的内存使用
- 及时清理临时文件
- 防止磁盘空间耗尽

### 性能优化

#### 内存管理

- 使用流式处理避免大文件内存占用
- 分块读取和写入文件数据
- 及时释放不再需要的资源

#### 并发控制

- 防止同一预留记录的并发合并
- 使用数据库事务保证数据一致性
- 实现适当的锁机制

## 数据库变更

### 新增 Repository 方法

#### IRoomUploadReservationRepository 接口

```rust
async fn update_upload_status(&self, reservation_id: i64, status: UploadStatus) -> Result<()>;
async fn consume_upload(&self, reservation_id: i64) -> Result<()>;
```

#### SqliteRoomUploadReservationRepository 实现

```rust
async fn update_upload_status(&self, reservation_id: i64, status: UploadStatus) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE room_upload_reservations
        SET upload_status = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
        status,
        reservation_id
    )
    .execute(&*self.pool)
    .await?;

    Ok(())
}

async fn consume_upload(&self, reservation_id: i64) -> Result<()> {
    let now = Utc::now().naive_utc();
    sqlx::query!(
        r#"
        UPDATE room_upload_reservations
        SET consumed_at = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
        now,
        reservation_id
    )
    .execute(&*self.pool)
    .await?;

    Ok(())
}
```

## 路由配置

在`crates/board/src/route/room.rs`中添加了新的路由：

```rust
.route(
    "/api/v1/rooms/{name}/uploads/chunks/complete",
    axum_post(crate::handlers::complete_file_merge),
)
```

## 文件结构

文件合并完成 API 端点的实现涉及以下文件：

### 核心实现文件

- `crates/board/src/handlers/chunked_upload.rs`: 主要处理逻辑
- `crates/board/src/models/room/chunk_upload.rs`: 数据模型定义
- `crates/board/src/models/room/upload_reservation.rs`: 预留记录模型
- `crates/board/src/repository/room_upload_reservation_repository.rs`:
  数据访问层

### 配置文件

- `crates/board/src/route/room.rs`: 路由配置

## 测试建议

### 单元测试

- 测试文件合并逻辑的正确性
- 测试哈希验证功能
- 测试错误处理逻辑

### 集成测试

- 测试完整的 API 端点流程
- 测试并发场景
- 测试大文件合并场景

### 性能测试

- 测试大文件合并的内存使用
- 测试并发合并的性能
- 测试文件系统 I/O 性能

## 使用示例

### 请求示例

```bash
curl -X POST "http://localhost:3000/api/v1/rooms/myroom/uploads/chunks/complete" \
  -H "Content-Type: application/json" \
  -d '{
    "reservation_id": 123,
    "final_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  }'
```

### 成功响应示例

```json
{
  "reservation_id": 123,
  "upload_status": "completed",
  "file_info": {
    "name": "document.pdf",
    "size": 2097152,
    "hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "path": "storage/rooms/456/document.pdf"
  }
}
```

## 后续改进建议

### 功能增强

- 支持断点续传的文件合并
- 支持多文件批量合并
- 支持合并进度查询

### 性能优化

- 实现并行分块合并
- 优化大文件处理性能
- 实现智能缓存机制

### 监控和日志

- 添加详细的操作日志
- 实现性能指标监控
- 添加异常告警机制

## 总结

文件合并完成 API 端点的实现为 Elizabeth
项目的分块上传功能提供了完整的闭环。该实现遵循了 RESTful API
设计原则，提供了完善的错误处理机制，并考虑了安全性和性能优化。通过该端点，用户可以可靠地将分块上传的文件合并为完整文件，为后续的文件处理和存储奠定了基础。
