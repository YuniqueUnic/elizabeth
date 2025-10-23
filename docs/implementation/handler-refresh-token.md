# Refresh Token Handler

## 1. 简介

Refresh Token Handler 是 Elizabeth 系统 JWT
刷新机制的核心组件，负责处理访问令牌的刷新、撤销和清理操作。该处理器实现了无状态的令牌刷新机制，支持令牌轮换和黑名单管理，确保系统的安全性和用户体验。主要交互方包括刷新令牌服务（`crates/board/src/services/refresh_token_service.rs`）、刷新令牌模型（`crates/board/src/models/room/refresh_token.rs`）和令牌黑名单仓储。

## 2. 数据模型（字段 & 类型 & 解释）

**RefreshTokenRequest
结构体**（`crates/board/src/models/room/refresh_token.rs:85`）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,  // 刷新令牌字符串
}
```

**RefreshTokenResponse
结构体**（`crates/board/src/models/room/refresh_token.rs:90`）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenResponse {
    pub access_token: String,           // 新的访问令牌
    pub refresh_token: String,         // 新的刷新令牌（可选，支持轮换）
    pub expires_at: String,            // 访问令牌过期时间
    pub refresh_expires_at: String,    // 刷新令牌过期时间
}
```

**LogoutRequest 结构体**（`crates/board/src/models/room/refresh_token.rs:95`）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,  // 要撤销的刷新令牌
}
```

**CleanupResponse
结构体**（`crates/board/src/models/room/refresh_token.rs:100`）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CleanupResponse {
    pub cleaned_count: i64,  // 清理的令牌数量
    pub success: bool,       // 清理是否成功
}
```

## 3. 不变式 & 验证逻辑（业务规则）

- **令牌有效性验证**：刷新令牌必须存在、未过期、未被撤销
- **访问令牌关联验证**：刷新令牌必须关联到有效的访问令牌
- **房间状态验证**：关联的房间必须仍然可访问（未过期、未关闭）
- **令牌轮换机制**：成功刷新后，旧的刷新令牌自动失效
- **黑名单管理**：撤销的令牌 JTI 被加入黑名单，防止重用
- **安全哈希存储**：刷新令牌以 SHA-256 哈希形式存储，不存储明文

## 4. 持久化 & 索引（实现细节）

**数据库表结构**：

```sql
-- 刷新令牌表
CREATE TABLE IF NOT EXISTS room_refresh_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    access_token_jti TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,     -- SHA-256 哈希值
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

-- 令牌黑名单表
CREATE TABLE IF NOT EXISTS token_blacklist (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    jti TEXT NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**索引和约束**：

- 主键：`id`（自增）
- 唯一约束：`token_hash` 和 `jti` 字段
- 外键约束：`room_id` 关联到 `rooms.id`，级联删除
- 性能索引：`room_id`、`access_token_jti`、`token_hash`、`expires_at` 字段

**ORM 使用**：使用 SQLx 的 `FromRow` trait 进行自动映射，通过
`SqliteRoomRefreshTokenRepository` 进行数据库操作。

## 5. API/Handlers（对外行为）

**核心端点列表**：

- `POST /api/v1/auth/refresh` - 刷新访问令牌
  - 输入：`RefreshTokenRequest { refresh_token: String }`
  - 输出：`RefreshTokenResponse { access_token: String, refresh_token: String, expires_at: String, refresh_expires_at: String }`
  - 功能：使用刷新令牌获取新的访问令牌和刷新令牌对

- `POST /api/v1/auth/logout` - 撤销刷新令牌
  - 输入：`LogoutRequest { refresh_token: String }`
  - 输出：`HttpResponse`（状态码 200 表示成功）
  - 功能：撤销指定的刷新令牌，将其加入黑名单

- `POST /api/v1/auth/cleanup` - 清理过期令牌
  - 输入：无（需要管理员权限）
  - 输出：`CleanupResponse { cleaned_count: i64, success: bool }`
  - 功能：清理过期的刷新令牌和黑名单条目

**请求/响应示例**：

```json
// 刷新令牌请求
POST /api/v1/auth/refresh
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}

// 响应
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2024-01-01T01:30:00",
  "refresh_expires_at": "2024-01-02T01:00:00"
}

// 撤销令牌请求
POST /api/v1/auth/logout
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

## 6. 令牌刷新机制（如何工作）

**刷新流程**：

1. **令牌验证**：验证刷新令牌的签名、有效性和数据库记录
2. **关联检查**：验证刷新令牌关联的访问令牌和房间状态
3. **令牌生成**：生成新的访问令牌和刷新令牌对
4. **令牌轮换**：将旧的刷新令牌标记为已撤销
5. **黑名单更新**：将旧的访问令牌 JTI 加入黑名单

**安全机制**：

- **令牌轮换**：每次刷新后，旧的刷新令牌立即失效
- **访问令牌黑名单**：撤销的访问令牌 JTI 被加入黑名单
- **哈希存储**：刷新令牌以哈希形式存储，防止泄露
- **过期清理**：定期清理过期的令牌和黑名单条目

## 7. 关键代码片段（无需粘全部，提供入口/关键函数）

**令牌刷新逻辑**（`crates/board/src/handlers/refresh_token.rs:12`）：

```rust
pub async fn refresh_token(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, HttpResponse> {
    let refresh_service = &app_state.refresh_token_service;

    let response = refresh_service
        .refresh_access_token(&request.refresh_token)
        .await
        .map_err(|e| {
            logrs::error!("Failed to refresh access token: {}", e);
            HttpResponse::Unauthorized().message("Invalid or expired refresh token")
        })?;

    Ok(Json(response))
}
```

**令牌撤销逻辑**（`crates/board/src/handlers/refresh_token.rs:44`）：

```rust
pub async fn revoke_token(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<LogoutRequest>,
) -> Result<HttpResponse, HttpResponse> {
    let refresh_service = &app_state.refresh_token_service;

    refresh_service
        .revoke_refresh_token(&request.refresh_token)
        .await
        .map_err(|e| {
            logrs::error!("Failed to revoke refresh token: {}", e);
            HttpResponse::InternalServerError().message("Failed to revoke token")
        })?;

    Ok(HttpResponse::Ok().message("Token revoked successfully"))
}
```

**清理过期令牌逻辑**（`crates/board/src/handlers/refresh_token.rs:62`）：

```rust
pub async fn cleanup_expired_tokens(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<CleanupResponse>, HttpResponse> {
    let refresh_service = &app_state.refresh_token_service;

    let cleaned_count = refresh_service
        .cleanup_expired_tokens()
        .await
        .map_err(|e| {
            logrs::error!("Failed to cleanup expired tokens: {}", e);
            HttpResponse::InternalServerError().message("Cleanup failed")
        })?;

    Ok(Json(CleanupResponse {
        cleaned_count,
        success: true,
    }))
}
```

## 8. 测试要点（单元/集成测试建议）

- **基础功能测试**：
  - 令牌刷新、撤销、清理的完整流程
  - 令牌轮换机制的正确性
  - 过期时间计算的正确性

- **安全测试**：
  - 无效刷新令牌的拒绝
  - 过期刷新令牌的拒绝
  - 撤销令牌的失效验证
  - 令牌重放攻击的防护

- **边界条件测试**：
  - 刷新令牌即将过期时的处理
  - 房间过期时的刷新行为
  - 并发刷新请求的处理

- **集成测试**：
  - 完整的认证流程：登录 → 刷新 → 撤销
  - 令牌轮换的安全性验证
  - 黑名单机制的有效性

## 9. 已知问题 / TODO / 改进建议

**P0 优先级**：

- **无**：当前实现已满足基本需求

**P1 优先级**：

- **批量操作支持**：支持批量撤销多个刷新令牌
- **刷新限制**：添加刷新频率限制，防止滥用

**P2 优先级**：

- **设备管理**：支持多设备登录和设备管理
- **刷新统计**：添加令牌使用统计和分析功能

## 10. 关联文档 / 代码位置

**源码路径**：

- 刷新令牌处理器：`crates/board/src/handlers/refresh_token.rs`
- 刷新令牌服务：`crates/board/src/services/refresh_token_service.rs`
- 刷新令牌模型：`crates/board/src/models/room/refresh_token.rs`
- 刷新令牌仓储：`crates/board/src/repository/room_refresh_token_repository.rs`
- 数据库迁移：`crates/board/migrations/002_refresh_tokens.sql`

**测试文件路径**：

- 单元测试：`crates/board/src/handlers/refresh_token.rs` 中的 `#[cfg(test)]` 块
- 集成测试：`crates/board/tests/api_integration_tests.rs`

**关联文档**：

- [model-session-jwt.md](./model-session-jwt.md) - JWT 会话管理详细说明
- [system-auth.md](./system-auth.md) - 认证系统详细说明
- [handler-token.md](./handler-token.md) - 令牌处理器详细说明
