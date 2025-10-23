# Session/JWT

## 1. 简介

Session/JWT 模型是 Elizabeth 系统的身份验证和会话管理核心，采用无状态的
JWT（JSON Web
Token）机制。系统没有传统的用户注册登录，而是通过房间访问令牌来管理会话。每个成功进入房间的用户都会获得一个短期有效的
JWT，该令牌包含了用户在房间内的权限信息。系统还支持刷新令牌机制，允许用户在访问令牌过期后自动获取新的令牌，提升用户体验。主要交互方包括房间处理器（`crates/board/src/handlers/rooms.rs`）、内容处理器（`crates/board/src/handlers/content.rs`）、
token
服务（`crates/board/src/services/token.rs`）和刷新令牌服务（`crates/board/src/services/refresh_token_service.rs`）。

## 2. 数据模型（字段 & 类型 & 解释）

**RoomToken 结构体**（`crates/board/src/models/room/token.rs:6`）：

```rust
pub struct RoomToken {
    pub id: Option<i64>,              // 主键，数据库记录 ID
    pub room_id: i64,                 // 关联的房间 ID
    pub jti: String,                  // JWT 的唯一标识符
    pub expires_at: NaiveDateTime,    // 令牌过期时间
    pub revoked_at: Option<NaiveDateTime>, // 令牌撤销时间
    pub created_at: NaiveDateTime,    // 令牌创建时间
}
```

**RoomTokenClaims 结构体**（`crates/board/src/services/token.rs:17`）：

```rust
pub struct RoomTokenClaims {
    pub sub: String,          // 主题，格式为 "room:{room_id}"
    pub room_id: i64,         // 房间 ID
    pub room_name: String,    // 房间名称（slug）
    pub permission: u8,       // 权限位标志（RoomPermission 的 bits）
    pub max_size: i64,        // 房间最大容量
    pub exp: i64,            // 过期时间戳
    pub iat: i64,            // 签发时间戳
    pub jti: String,         // JWT 唯一标识符
}
```

**数据库映射**：对应 `crates/board/migrations/001_initial_schema.sql` 中的
`room_tokens` 表，以及 `crates/board/migrations/002_refresh_tokens.sql` 中的
`room_refresh_tokens` 和 `token_blacklist` 表。

**RoomRefreshToken
结构体**（`crates/board/src/models/room/refresh_token.rs:6`）：

```rust
/// 房间刷新令牌数据模型
/// 用于存储和管理 JWT 刷新令牌的信息
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RoomRefreshToken {
    pub id: Option<i64>,                    // 主键 ID
    pub room_id: i64,                       // 关联的房间 ID
    pub access_token_jti: String,           // 关联的访问令牌 JTI
    pub token_hash: String,                 // 刷新令牌的 SHA-256 哈希值（不存储明文）
    pub expires_at: NaiveDateTime,          // 刷新令牌过期时间
    pub created_at: NaiveDateTime,          // 创建时间
    pub last_used_at: Option<NaiveDateTime>, // 最后使用时间
    pub is_revoked: bool,                   // 是否已撤销
}
```

**TokenBlacklistEntry
结构体**（`crates/board/src/models/room/refresh_token.rs:75`）：

```rust
/// 令牌黑名单条目
/// 用于存储被撤销的令牌 JTI，防止重用
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct TokenBlacklistEntry {
    pub id: Option<i64>,           // 主键 ID
    pub jti: String,               // 被撤销的令牌 JTI
    pub expires_at: NaiveDateTime,  // 令牌原过期时间
    pub created_at: NaiveDateTime,  // 创建时间
}
```

## 3. 不变式 & 验证逻辑（业务规则）

- **令牌唯一性**：每个 JWT 的 `jti`（JWT ID）在系统中必须唯一
- **时效性控制**：JWT 默认有效期 30 分钟，但不超过房间过期时间
- **房间关联**：JWT 只能用于其指定的房间，不能跨房间使用
- **权限绑定**：JWT 中包含的权限在签发时就已确定，不会动态变化
- **撤销机制**：令牌可以通过设置 `revoked_at` 字段来主动撤销
- **过期验证**：系统同时检查 JWT 的 `exp` 字段和数据库中的
  `expires_at`、`revoked_at` 字段
- **刷新令牌轮换**：每次成功刷新后，旧的刷新令牌自动失效
- **黑名单机制**：撤销的访问令牌 JTI 被加入黑名单，防止重用
- **安全存储**：刷新令牌以 SHA-256 哈希形式存储，不存储明文
- **令牌关联**：刷新令牌必须关联到有效的访问令牌 JTI

## 4. 持久化 & 索引（实现细节）

**数据库表结构**：

```sql
-- 访问令牌表
CREATE TABLE IF NOT EXISTS room_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- 主键，自增 ID，对应 RoomToken.id 字段
    room_id INTEGER NOT NULL,
    jti TEXT NOT NULL UNIQUE,              -- JWT 唯一标识
    expires_at DATETIME NOT NULL,          -- 过期时间
    revoked_at DATETIME,                   -- 撤销时间
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

-- 刷新令牌表
CREATE TABLE IF NOT EXISTS room_refresh_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    access_token_jti TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,       -- SHA-256 哈希值
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

-- 索引优化查询性能
CREATE INDEX IF NOT EXISTS idx_room_tokens_room_id ON room_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_tokens_expires_at ON room_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_room_id ON room_refresh_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_access_jti ON room_refresh_tokens(access_token_jti);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_token_hash ON room_refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_expires_at ON room_refresh_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_token_blacklist_jti ON token_blacklist(jti);
CREATE INDEX IF NOT EXISTS idx_token_blacklist_expires_at ON token_blacklist(expires_at);
```

**索引和约束**：

- 主键：`id`（自增）
- 唯一约束：`jti`、`token_hash` 字段
- 外键约束：`room_id` 关联到 `rooms.id`，级联删除
- 性能索引：`room_id`、`expires_at`、`access_token_jti`、`token_hash` 字段

**ORM 使用**：使用 SQLx 的 `FromRow` trait 进行自动映射，通过
`SqliteRoomTokenRepository` 和 `SqliteRoomRefreshTokenRepository`
进行数据库操作。

## 5. API/Handlers（对外行为）

**核心端点列表**：

- `POST /api/v1/rooms/{name}/tokens` - 签发房间访问令牌
  - 输入：`CreateTokenRequest { password: Option<String>, edit: bool, share: bool, delete: bool, max_times_enter_room: Option<i32>, ttl_seconds: Option<i64> }`
  - 输出：`CreateTokenResponse { token: String, jti: String, permission: u8, expires_at: String }`
  - 功能：验证密码并创建新的 JWT
  - 实际路由：`crates/board/src/route/room.rs:18`
  - 处理函数：`crates/board/src/handlers/rooms.rs:issue_token`

- `GET /api/v1/rooms/{name}/tokens` - 列出房间令牌
  - 查询参数：`token: String`
  - 输出：`ListTokensResponse { tokens: Vec<TokenInfo> }`
  - 实际路由：`crates/board/src/route/room.rs:19`
  - 处理函数：`crates/board/src/handlers/rooms.rs:list_tokens`

- `POST /api/v1/rooms/{name}/tokens/validate` - 验证令牌
  - 输入：`ValidateTokenRequest { token: String }`
  - 输出：`ValidateTokenResponse { valid: bool, room_id: i64, room_name: String, permission: u8, can_view: bool, can_edit: bool, can_share: bool, can_delete: bool, expires_at: Option<String> }`
  - 实际路由：`crates/board/src/route/room.rs:21`
  - 处理函数：`crates/board/src/handlers/rooms.rs:validate_token`

- `DELETE /api/v1/rooms/{name}/tokens/{jti}` - 撤销令牌
  - 查询参数：`token: String`
  - 输出：成功消息
  - 实际路由：`crates/board/src/route/room.rs:24`
  - 处理函数：`crates/board/src/handlers/rooms.rs:revoke_token`

- `POST /api/v1/auth/refresh` - 刷新访问令牌
  - 输入：`RefreshTokenRequest { refresh_token: String }`
  - 输出：`RefreshTokenResponse { access_token: String, refresh_token: String, expires_at: String, refresh_expires_at: String }`
  - 实际路由：`crates/board/src/route/auth.rs`
  - 处理函数：`crates/board/src/handlers/refresh_token.rs:refresh`

- `POST /api/v1/auth/logout` - 撤销刷新令牌
  - 输入：`LogoutRequest { refresh_token: String }`
  - 输出：成功消息
  - 实际路由：`crates/board/src/route/auth.rs`
  - 处理函数：`crates/board/src/handlers/refresh_token.rs:logout`

**数据结构定义**（基于实际代码实现）：

```rust
// 创建令牌请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTokenRequest {
    pub password: Option<String>,
    pub edit: bool,
    pub share: bool,
    pub delete: bool,
    pub max_times_enter_room: Option<i32>,
    pub ttl_seconds: Option<i64>,
}

// 创建令牌响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTokenResponse {
    pub token: String,
    pub jti: String,
    pub permission: u8,
    pub expires_at: String,  // ISO 8601 格式
}

// 验证令牌请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidateTokenRequest {
    pub token: String,
}

// 验证令牌响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidateTokenResponse {
    pub valid: bool,
    pub room_id: i64,
    pub room_name: String,
    pub permission: u8,
    pub can_view: bool,
    pub can_edit: bool,
    pub can_share: bool,
    pub can_delete: bool,
    pub expires_at: Option<String>,
}

// 刷新令牌请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

// 刷新令牌响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
    pub refresh_expires_at: String,
}

// 登出请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}
```

**请求/响应示例**：

```json
// 签发令牌请求
POST /api/v1/rooms/myroom/tokens
{
  "password": "secret123"
}

// 响应
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "claims": {
    "sub": "room:1",
    "room_id": 1,
    "room_name": "myroom",
    "permission": 15,
    "max_size": 10485760,
    "exp": 1704067200,
    "iat": 1704065400,
    "jti": "550e8400-e29b-41d4-a716-446655440000"
  },
  "expires_at": "2024-01-01T01:00:00"
}
```

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
```

## 6. JWT 与权限（如何生成/校验）

**JWT 生成流程**：

1. **权限提取**：从 `room.permission` 获取权限位标志（`room.permission.bits()`）
2. **时间计算**：计算令牌过期时间，取系统默认（30 分钟）和房间过期时间的较小值
3. **唯一标识**：生成 UUID 作为 JWT 的 `jti`
4. **签名编码**：使用 HS256 算法和服务器密钥签名

**JWT 校验流程**：

1. **签名验证**：验证 JWT 签名的有效性
2. **时间检查**：检查 `exp` 字段是否过期（允许 5 秒时钟偏移）
3. **黑名单检查**：查询 `token_blacklist` 表确认令牌 JTI 未被列入黑名单
4. **数据库验证**：查询 `room_tokens` 表确认令牌未被撤销且未过期
5. **房间状态**：验证关联房间仍然可进入（未过期、未关闭、未超限）

**刷新令牌验证流程**：

1. **哈希验证**：验证刷新令牌的 SHA-256 哈希值
2. **有效性检查**：确认刷新令牌未过期、未被撤销
3. **关联验证**：验证刷新令牌关联的访问令牌 JTI 仍然有效
4. **房间状态**：验证关联房间仍然可进入
5. **令牌轮换**：生成新的访问令牌和刷新令牌对，撤销旧的令牌

**权限验证**：

```rust
// 从 JWT claims 提取权限
let permission = claims.as_permission();
// 检查具体权限
if permission.can_edit() {
    // 允许编辑操作
}
```

## 7. 关键代码片段（无需粘全部，提供入口/关键函数）

**JWT 签发逻辑**（`crates/board/src/services/token.rs:61`）：

```rust
pub fn issue(&self, room: &Room) -> Result<(String, RoomTokenClaims)> {
    if room.is_expired() {
        return Err(anyhow!("room already expired"));
    }

    let now = Utc::now();
    let mut exp = now + self.ttl;

    // 确保不超过房间过期时间
    if let Some(room_expire) = room.expire_at {
        let room_expire = room_expire - chrono::Duration::seconds(self.leeway);
        if room_expire <= now.naive_utc() {
            return Err(anyhow!("room expires too soon to issue token"));
        }
        let room_expire_dt = chrono::DateTime::<Utc>::from_naive_utc_and_offset(room_expire, Utc);
        if exp > room_expire_dt {
            exp = room_expire_dt;
        }
    }

    let jti = Uuid::new_v4().to_string();
    let claims = RoomTokenClaims {
        sub: format!("room:{}", room.id.unwrap_or_default()),
        room_id: room.id.ok_or_else(|| anyhow!("room id missing"))?,
        room_name: room.slug.clone(),
        permission: room.permission.bits(),
        max_size: room.max_size,
        exp: exp.timestamp(),
        iat: now.timestamp(),
        jti,
    };

    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(self.secret.as_bytes()),
    )?;

    Ok((token, claims))
}
```

**令牌验证逻辑**（`crates/board/src/handlers/token.rs:18`）：

```rust
pub async fn verify_room_token(
    app_state: Arc<AppState>,
    room_name: &str,
    token_str: &str,
) -> Result<VerifiedRoomToken, HttpResponse> {
    // 1. 解码和验证 JWT
    let claims = app_state
        .token_service
        .decode(token_str)
        .map_err(|_| HttpResponse::Unauthorized().message("Token is invalid or expired"))?;

    // 2. 验证房间匹配
    if claims.room_name != room_name {
        return Err(HttpResponse::Unauthorized().message("Token not issued for this room"));
    }

    // 3. 验证房间状态
    let room = room_repo
        .find_by_name(room_name)
        .await?
        .ok_or_else(|| HttpResponse::NotFound().message("Room not found"))?;

    if room.id != Some(claims.room_id) {
        return Err(HttpResponse::Unauthorized().message("Token room mismatch"));
    }
    if room.is_expired() {
        return Err(HttpResponse::Unauthorized().message("Room expired"));
    }

    // 4. 验证令牌数据库记录
    let record = token_repo
        .find_by_jti(&claims.jti)
        .await?
        .ok_or_else(|| HttpResponse::Unauthorized().message("Token revoked or not found"))?;

    if !record.is_active() {
        return Err(HttpResponse::Unauthorized().message("Token revoked or expired"));
    }

    Ok(VerifiedRoomToken {
        room,
        claims,
        record,
    })
}
```

**RoomToken 活跃状态检查**（`crates/board/src/models/room/token.rs:29`）：

```rust
pub fn is_active(&self) -> bool {
    self.revoked_at.is_none() && self.expires_at > Utc::now().naive_utc()
}
```

## 8. 测试要点（单元/集成测试建议）

- **基础功能测试**：
  - JWT 签发、验证、撤销的完整流程
  - 权限信息的正确编码和解码
  - 过期时间计算的正确性

- **安全测试**：
  - 无效签名的 JWT 拒绝
  - 过期 JWT 的拒绝
  - 跨房间 JWT 的拒绝
  - 撤销令牌的失效验证

- **边界条件测试**：
  - 房间即将过期时的令牌签发
  - 时钟偏移情况下的验证
  - 并发令牌签发和撤销

- **集成测试**：
  - 完整的房间访问流程：创建房间 → 获取令牌 → 访问内容
  - 令牌续签流程
  - 权限变更对现有令牌的影响

## 9. 已知问题 / TODO / 改进建议

**P0 优先级**：

- **无**：令牌刷新机制已实现，基本功能完善

**P1 优先级**：

- **令牌黑名单缓存**：将被撤销的令牌缓存到内存中，减少数据库查询
- **审计日志增强**：记录令牌签发、使用、撤销的详细审计信息
- **批量操作支持**：支持批量撤销多个刷新令牌

**P2 优先级**：

- **令牌分级管理**：支持不同有效期和权限级别的令牌类型
- **设备管理**：支持多设备登录和设备管理
- **刷新限制**：添加刷新频率限制，防止滥用

## 10. 关联文档 / 代码位置

**源码路径**：

- 令牌模型：`crates/board/src/models/room/token.rs`
- 刷新令牌模型：`crates/board/src/models/room/refresh_token.rs`
- 令牌服务：`crates/board/src/services/token.rs`
- 刷新令牌服务：`crates/board/src/services/refresh_token_service.rs`
- 令牌处理器：`crates/board/src/handlers/token.rs`
- 刷新令牌处理器：`crates/board/src/handlers/refresh_token.rs`
- 令牌仓储：`crates/board/src/repository/room_token_repository.rs`
- 刷新令牌仓储：`crates/board/src/repository/room_refresh_token_repository.rs`
- 数据库迁移：`crates/board/migrations/001_initial_schema.sql`
- 刷新令牌迁移：`crates/board/migrations/002_refresh_tokens.sql`

**测试文件路径**：

- 单元测试：`crates/board/src/services/token.rs` 中的 `#[cfg(test)]` 块
- 刷新令牌测试：`crates/board/src/handlers/refresh_token.rs` 中的 `#[cfg(test)]`
  块
- 集成测试：`crates/board/tests/api_integration_tests.rs`

**关联文档**：

- [model-room.md](./model-room.md) - 房间模型详细说明
- [model-permissions.md](./model-permissions.md) - 权限系统详细说明
- [model-file.md](./model-file.md) - 文件内容管理
- [handler-refresh-token.md](./handler-refresh-token.md) -
  刷新令牌处理器详细说明
