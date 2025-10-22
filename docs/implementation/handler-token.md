# 令牌处理器 (Token Handler)

## 1. 简介

令牌处理器是 Elizabeth 系统的安全核心组件，负责处理 JWT
令牌的签发、验证、撤销和管理功能。该处理器实现了基于 JWT
的无状态身份验证机制，支持房间访问控制、权限管理和会话管理。处理器采用 HS256
算法进行令牌签名，支持令牌续签、批量撤销和过期管理。主要交互方包括房间管理器、权限验证模块、数据库存储层和各个业务处理器。

## 2. 数据模型

### 令牌声明模型 (RoomTokenClaims)

- sub: String — 主题，格式为 "room:{room_id}"
- room_id: i64 — 房间 ID
- room_name: String — 房间名称（实际上是 slug）
- permission: u8 — 权限位掩码
- max_size: i64 — 房间最大容量限制
- exp: i64 — 过期时间（Unix 时间戳）
- iat: i64 — 签发时间（Unix 时间戳）
- jti: String — 令牌唯一标识符（UUID）

### 令牌记录模型 (RoomToken)

- jti: String — 令牌唯一标识符
- room_id: i64 — 关联的房间 ID
- expires_at: NaiveDateTime — 过期时间
- revoked_at: Option<NaiveDateTime> — 撤销时间
- created_at: NaiveDateTime — 创建时间

### 令牌服务配置

- secret: Arc<String> — JWT 签名密钥
- ttl: Duration — 令牌默认生存时间（默认 30 分钟）
- leeway: i64 — 时间容差（默认 5 秒）

### 请求/响应模型

- IssueTokenRequest: 令牌签发请求（密码、现有令牌）
- IssueTokenResponse: 令牌签发响应（令牌、声明、过期时间）
- ValidateTokenRequest: 令牌验证请求
- ValidateTokenResponse: 令牌验证响应（声明信息）

> 数据库表：`room_tokens`（迁移文件：`crates/board/migrations/005_create_room_tokens_table.sql`）

## 3. 不变式 & 验证逻辑

### 业务规则

- 令牌必须包含有效的房间 ID 和权限信息
- 令牌过期时间不能超过房间过期时间
- 令牌签发时房间必须处于可进入状态
- 令牌撤销后立即失效，无法恢复
- 续签令牌时会自动撤销旧令牌
- 令牌验证时检查数据库中的撤销状态
- 每个令牌都有唯一的 JTI 标识符

### 验证逻辑

- JWT 签名验证（使用 HS256 算法）
- 令牌过期时间检查（包含 leeway 容差）
- 房间存在性和状态验证
- 令牌撤销状态检查
- 权限级别验证和解析

### 安全约束

- 签名密钥必须保密且足够复杂
- 令牌 TTL 不能过长（默认 30 分钟）
- 房间过期时令牌自动失效
- 支持令牌的主动撤销机制

## 4. 持久化 & 索引

### 数据库表结构

```sql
CREATE TABLE IF NOT EXISTS room_tokens (
    jti TEXT PRIMARY KEY,
    room_id INTEGER NOT NULL,
    expires_at DATETIME NOT NULL,
    revoked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

### 索引设计

- 主键索引：`jti` 字段的主键索引
- 房间索引：`room_id` 字段优化按房间查询令牌
- 过期时间索引：`expires_at` 字段优化过期令牌清理
- 撤销时间索引：`revoked_at` 字段优化撤销令牌查询

### 清理策略

- 定期清理过期的令牌记录
- 撤销的令牌保留一段时间用于审计

## 5. API/Handlers

### 签发房间访问令牌

- **POST** `/api/v1/rooms/{name}/tokens`
- 请求参数：房间名称、密码或现有令牌
- 响应：JWT 令牌、声明信息、过期时间
- 错误码：400（参数错误）、403（权限不足）、404（房间不存在）

### 验证房间访问令牌

- **POST** `/api/v1/rooms/{name}/tokens/validate`
- 请求参数：房间名称、令牌
- 响应：令牌声明信息
- 错误码：401（令牌无效）、404（房间不存在）

### 获取房间令牌列表

- **GET** `/api/v1/rooms/{name}/tokens`
- 请求参数：房间名称、管理员令牌
- 响应：房间所有令牌的列表
- 错误码：401（令牌无效）、404（房间不存在）

### 撤销房间令牌

- **DELETE** `/api/v1/rooms/{name}/tokens/{jti}`
- 请求参数：房间名称、目标 JTI、管理员令牌
- 响应：撤销结果
- 错误码：401（令牌无效）、404（房间不存在）

### 请求示例

```json
// 签发令牌请求
POST /api/v1/rooms/myroom/tokens
{
  "password": "secret123",
  "token": null
}

// 签发令牌响应
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2023-12-01T10:30:00",
  "claims": {
    "sub": "room:123",
    "room_id": 123,
    "room_name": "myroom",
    "permission": 15,
    "max_size": 10485760,
    "exp": 1701436200,
    "iat": 1701434400,
    "jti": "550e8400-e29b-41d4-a716-446655440000"
  }
}

// 验证令牌请求
POST /api/v1/rooms/myroom/tokens/validate
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

## 6. JWT 与权限

### 令牌签发流程

```rust
pub fn issue(&self, room: &Room) -> Result<(String, RoomTokenClaims)> {
    // 检查房间状态
    if room.is_expired() {
        return Err(anyhow!("room already expired"));
    }

    // 计算过期时间
    let now = Utc::now();
    let mut exp = now + self.ttl;

    // 考虑房间过期时间
    if let Some(room_expire) = room.expire_at {
        let room_expire = room_expire - chrono::Duration::seconds(self.leeway);
        if room_expire <= now.naive_utc() {
            return Err(anyhow!("room expires too soon to issue token"));
        }
        if exp > room_expire_dt {
            exp = room_expire_dt;
        }
    }

    // 创建声明
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

    // 签名令牌
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(self.secret.as_bytes()),
    )?;

    Ok((token, claims))
}
```

### 令牌验证流程

```rust
pub async fn verify_room_token(
    app_state: Arc<AppState>,
    room_name: &str,
    token_str: &str,
) -> Result<VerifiedRoomToken, HttpResponse> {
    // 1. 解码和验证 JWT 签名
    let claims = app_state
        .token_service
        .decode(token_str)
        .map_err(|_| HttpResponse::Unauthorized().message("Token is invalid or expired"))?;

    // 2. 验证令牌与房间的匹配性
    if claims.room_name != room_name {
        return Err(HttpResponse::Unauthorized().message("Token not issued for this room"));
    }

    // 3. 验证房间存在性和基本状态
    let room_repo = SqliteRoomRepository::new(app_state.db_pool.clone());
    let room = room_repo
        .find_by_name(room_name)
        .await
        .map_err(|e| HttpResponse::InternalServerError().message(format!("Database error: {e}")))?
        .ok_or_else(|| HttpResponse::NotFound().message("Room not found"))?;

    // 4. 验证房间 ID 匹配
    if room.id != Some(claims.room_id) {
        return Err(HttpResponse::Unauthorized().message("Token room mismatch"));
    }

    // 5. 验证房间过期状态（房间时间过期检查）
    if room.is_expired() {
        return Err(HttpResponse::Unauthorized().message("Room expired"));
    }

    // 6. 验证房间可进入状态（包括房间状态、进入次数限制等）
    // 注意：这个检查在 room.can_enter() 中包含：
    // - 房间未过期（is_expired）
    // - 房间状态不为 Close
    // - 当前进入次数未超过最大限制
    if !room.can_enter() {
        return Err(HttpResponse::Unauthorized().message("Room cannot be entered"));
    }

    // 7. 验证令牌数据库记录存在性
    let token_repo = SqliteRoomTokenRepository::new(app_state.db_pool.clone());
    let record = token_repo
        .find_by_jti(&claims.jti)
        .await
        .map_err(|e| HttpResponse::InternalServerError().message(format!("Database error: {e}")))?
        .ok_or_else(|| HttpResponse::Unauthorized().message("Token revoked or not found"))?;

    // 8. 验证令牌活跃状态（未被撤销且未过期）
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

**房间状态验证详细说明**：

在令牌验证过程中，房间状态验证包含以下几个关键检查：

1. **房间存在性验证**：确保房间在数据库中存在
2. **房间 ID 匹配验证**：确保令牌中的房间 ID 与实际房间 ID 一致
3. **房间过期时间验证**：通过 `room.is_expired()` 检查房间是否已过期
4. **房间可进入状态验证**：通过 `room.can_enter()` 进行综合状态检查：
   - 房间状态不为 `RoomStatus::Close`
   - 当前进入次数 (`current_times_entered`) 未超过最大限制 (`max_times_entered`)
   - 房间未过期（双重保险检查）

**验证顺序的重要性**：

- 先验证 JWT 签名和基本匹配性，快速拒绝无效请求
- 然后验证房间状态，避免为不可用的房间进行数据库查询
- 最后验证令牌数据库记录，确保令牌未被撤销

## 7. 关键代码片段

### 令牌服务实现 (crates/board/src/services/token.rs:48)

```rust
impl RoomTokenService {
    pub fn new(secret: Arc<String>) -> Self {
        Self::with_ttl(secret, Duration::minutes(DEFAULT_TOKEN_TTL_MINUTES))
    }

    pub fn with_ttl(secret: Arc<String>, ttl: Duration) -> Self {
        Self {
            secret,
            ttl,
            leeway: DEFAULT_LEEWAY_SECONDS,
        }
    }

    pub fn decode(&self, token: &str) -> Result<RoomTokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = self.leeway as u64;
        let data = jsonwebtoken::decode::<RoomTokenClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        ).context("invalid token")?;

        Ok(data.claims)
    }
}
```

### 签发令牌处理器 (crates/board/src/handlers/rooms.rs:241)

```rust
pub async fn issue_token(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<IssueTokenRequest>,
) -> HandlerResult<IssueTokenResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    let mut previous_jti = None;
    let room = if let Some(token) = payload.token.as_deref() {
        // 使用现有令牌续签
        let verified = verify_room_token(app_state.clone(), &name, token).await?;
        previous_jti = Some(verified.record.jti.clone());
        verified.room
    } else {
        // 使用密码验证
        let repository = SqliteRoomRepository::new(app_state.db_pool.clone());
        let Some(room) = repository.find_by_name(&name).await.map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Database error: {e}"))
        })? else {
            return Err(HttpResponse::NotFound().message("Room not found"));
        };

        if let Some(expected_password) = room.password.as_ref()
            && payload.password.as_deref() != Some(expected_password.as_str())
        {
            return Err(HttpResponse::Forbidden().message("Invalid room password"));
        }

        room
    };

    if !room.can_enter() {
        return Err(HttpResponse::Forbidden().message("Room cannot be entered"));
    }

    // 签发新令牌
    let (token, claims) = app_state
        .token_service
        .issue(&room)
        .map_err(|e| HttpResponse::Forbidden().message(e.to_string()))?;

    // 保存令牌记录
    let record = RoomToken::new(claims.room_id, claims.jti.clone(), claims.expires_at());
    let token_repo = SqliteRoomTokenRepository::new(app_state.db_pool.clone());
    token_repo.create(&record).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to persist token: {e}"))
    })?;

    // 撤销旧令牌
    if let Some(jti) = previous_jti {
        token_repo.revoke(&jti).await.map_err(|e| {
            HttpResponse::InternalServerError().message(format!("Failed to revoke old token: {e}"))
        })?;
    }

    Ok(Json(IssueTokenResponse {
        token,
        expires_at: claims.expires_at(),
        claims,
    }))
}
```

### 撤销令牌处理器 (crates/board/src/handlers/rooms.rs:467)

```rust
pub async fn revoke_token(
    Path((name, target_jti)): Path<(String, String)>,
    Query(query): Query<TokenQuery>,
    State(app_state): State<Arc<AppState>>,
) -> HandlerResult<RevokeTokenResponse> {
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().message("Invalid room name"));
    }

    // 验证请求者权限
    let _verified = verify_room_token(app_state.clone(), &name, &query.token).await?;

    // 撤销目标令牌
    let token_repo = SqliteRoomTokenRepository::new(app_state.db_pool.clone());
    let revoked = token_repo.revoke(&target_jti).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Failed to revoke token: {e}"))
    })?;

    Ok(Json(RevokeTokenResponse { revoked }))
}
```

## 8. 测试要点

### 单元测试建议

- 测试令牌签发和验证的完整流程
- 测试令牌过期时间计算逻辑
- 测试权限位掩码的编码和解码
- 测试令牌撤销机制
- 测试房间过期对令牌的影响

### 集成测试建议

- 完整的令牌生命周期：签发 → 验证 → 续签 → 撤销
- 并发令牌操作测试
- 跨房间令牌访问控制测试
- 令牌泄露场景的安全测试
- 大量令牌的性能测试

### 安全测试建议

- JWT 签名伪造攻击测试
- 令牌篡改检测测试
- 重放攻击防护测试
- 时间攻击防护测试
- 密钥泄露影响评估

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

- **令牌刷新机制**：当前缺乏自动令牌刷新，建议实现滑动窗口刷新机制
- **令牌黑名单**：撤销的令牌仅标记为撤销，建议实现全局黑名单机制

### P1 优先级

- **令牌审计日志**：缺乏详细的令牌操作日志，建议添加安全审计功能
- **密钥轮换机制**：缺乏密钥轮换支持，建议实现无缝密钥更新

### P2 优先级

- **令牌模板功能**：缺乏预设令牌模板，建议支持常用权限配置
- **令牌统计分析**：缺乏令牌使用统计，建议添加使用量分析功能

## 10. 关联文档 / 代码位置

### 源码路径

- 处理器实现：`crates/board/src/handlers/rooms.rs:241-484`
- 令牌验证：`crates/board/src/handlers/token.rs:18-62`
- 令牌服务：`crates/board/src/services/token.rs:48-121`
- 路由定义：`crates/board/src/route/room.rs:17-26`

### 数据库相关

- 迁移文件：`crates/board/migrations/005_create_room_tokens_table.sql`
- 仓库实现：`crates/board/src/repository/room_token_repository.rs`

### 测试文件

- 集成测试：`crates/board/tests/api_integration_tests.rs`
- 服务测试：`crates/board/src/services/token.rs`（单元测试）

### 相关文档

- [房间模型文档](model-room.md)
- [权限模型文档](model-permissions.md)
- [会话 JWT 文档](model-session-jwt.md)
- [上传处理器文档](handler-upload.md)
- [下载处理器文档](handler-download.md)
- [房间管理文档](handler-admin.md)
