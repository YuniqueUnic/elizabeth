# 认证系统 (Authentication System)

## 1. 简介

Elizabeth 认证系统采用基于 JWT (JSON Web Token)
的无状态认证机制，与传统的用户注册登录模式不同，系统以房间为中心构建认证体系。每个房间都有独立的访问控制，用户通过房间密码获取
JWT 令牌，令牌中包含房间权限信息，用于后续 API
访问控制。系统支持管理员令牌和访客令牌两种类型，提供细粒度的权限管理。

主要交互方包括：

- 房间处理器 (`crates/board/src/handlers/rooms.rs`) - 房间创建和进入
- 内容处理器 (`crates/board/src/handlers/content.rs`) - 文件操作权限验证
- 令牌服务 (`crates/board/src/services/token.rs`) - JWT 生成和验证
- 权限模型 (`crates/board/src/models/room/permission.rs`) - 权限定义和检查

## 2. 数据模型

### JWT Claims 结构

```rust
// 房间令牌声明 ([`crates/board/src/services/token.rs:17`](crates/board/src/services/token.rs:17))
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomTokenClaims {
    pub sub: String,        // 主题：格式 "room:{room_id}"
    pub room_id: i64,       // 房间 ID
    pub room_name: String,  // 房间名称 (slug)
    pub permission: u8,     // 权限位掩码
    pub max_size: i64,      // 房间最大容量
    pub exp: i64,           // 过期时间 (Unix 时间戳)
    pub iat: i64,           // 签发时间 (Unix 时间戳)
    pub jti: String,        // JWT ID (UUID)
}
```

### 令牌服务配置

```rust
// 令牌服务结构 ([`crates/board/src/services/token.rs:41`](crates/board/src/services/token.rs:41))
#[derive(Clone)]
pub struct RoomTokenService {
    secret: Arc<String>,    // 签名密钥
    ttl: Duration,          // 令牌有效期
    leeway: i64,            // 时钟容差秒数
}
```

### 数据库令牌记录

```sql
-- room_tokens 表结构
CREATE TABLE IF NOT EXISTS room_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    jti TEXT NOT NULL UNIQUE,    -- JWT ID
    expires_at DATETIME NOT NULL,
    revoked_at DATETIME,         -- 撤销时间
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
```

### 字段类型 & 解释

- `sub: String` - JWT 标准字段，标识令牌主体
- `room_id: i64` - 关联的房间 ID
- `permission: u8` - 位掩码形式的权限标识
- `exp: i64` - Unix 时间戳格式的过期时间
- `jti: String` - UUID 格式的令牌唯一标识符
- `leeway: i64` - 时钟容差，默认 5 秒

## 3. 不变式 & 验证逻辑

### 业务规则

1. **房间状态验证**: 只能为未过期且未关闭的房间签发令牌
2. **权限继承**: 令牌权限不能超过房间配置的权限
3. **时间约束**: 令牌有效期不能超过房间过期时间
4. **唯一性保证**: 每个 JWT ID (JTI) 必须唯一
5. **最小有效期**: 令牌有效期至少 5 秒，防止立即过期

### 验证逻辑

```rust
// 房间过期检查 ([`crates/board/src/services/token.rs:62`](crates/board/src/services/token.rs:62))
if room.is_expired() {
    return Err(anyhow!("room already expired"));
}

// 时间约束验证 ([`crates/board/src/services/token.rs:69`](crates/board/src/services/token.rs:69))
if let Some(room_expire) = room.expire_at {
    let room_expire = room_expire - chrono::Duration::seconds(self.leeway);
    if room_expire <= now.naive_utc() {
        return Err(anyhow!("room expires too soon to issue token"));
    }
    if exp > room_expire_dt {
        exp = room_expire_dt;
    }
}
```

### 权限验证流程

1. **令牌解析**: 验证 JWT 签名和格式
2. **时间检查**: 验证令牌是否在有效期内
3. **房间状态**: 检查关联房间的当前状态
4. **权限匹配**: 验证令牌权限是否满足操作要求

## 4. 持久化 & 索引

### 令牌持久化策略

- **主动存储**: 签发令牌时同时保存到数据库
- **被动清理**: 通过定时任务清理过期令牌
- **撤销支持**: 通过 `revoked_at` 字段支持令牌撤销

### 索引设计

```sql
-- 令牌查询优化索引
CREATE INDEX IF NOT EXISTS idx_room_tokens_room_id ON room_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_tokens_expires_at ON room_tokens(expires_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_room_tokens_jti ON room_tokens(jti);
```

### 令牌生命周期管理

```rust
// 令牌签出逻辑
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
```

## 5. API/Handlers

### 认证相关端点

- `POST /api/v1/rooms` - 创建房间（管理员令牌）
- `POST /api/v1/rooms/{name}/enter` - 进入房间获取访客令牌
- `POST /api/v1/rooms/{name}/logout` - 撤销令牌（可选实现）

### 令牌验证中间件

```rust
// 令牌验证函数 ([`crates/board/src/handlers/mod.rs`](crates/board/src/handlers/mod.rs))
pub async fn verify_room_token(
    app_state: Arc<AppState>,
    room_name: &str,
    token: &str,
) -> Result<VerifiedToken, HttpResponse>
```

### 权限检查模式

```rust
// 权限验证示例
ensure_permission(
    &verified.claims,
    verified.room.permission.can_view(),
    ContentPermission::View,
)?;
```

## 6. JWT 与权限

### 签名算法

- **算法**: HS256 (HMAC-SHA256)
- **密钥管理**: 通过环境变量或配置文件管理
- **密钥轮换**: 支持热更新密钥（需要重启服务）

### 令牌生成

```rust
// JWT 签名 ([`crates/board/src/services/token.rs:99`](crates/board/src/services/token.rs:99))
let token = jsonwebtoken::encode(
    &Header::new(Algorithm::HS256),
    &claims,
    &EncodingKey::from_secret(self.secret.as_bytes()),
)
.context("failed to sign room token")?;
```

### 令牌验证

```rust
// JWT 解码和验证 ([`crates/board/src/services/token.rs:109`](crates/board/src/services/token.rs:109))
pub fn decode(&self, token: &str) -> Result<RoomTokenClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = self.leeway as u64;
    let data = jsonwebtoken::decode::<RoomTokenClaims>(
        token,
        &DecodingKey::from_secret(self.secret.as_bytes()),
        &validation,
    )
    .context("invalid token")?;

    Ok(data.claims)
}
```

### 权限位掩码

```rust
// 权限定义 ([`crates/board/src/models/room/permission.rs:30`](crates/board/src/models/room/permission.rs:30))
pub struct RoomPermission: u8 {
    const VIEW_ONLY = 1;    // 查看权限：0b0001
    const EDITABLE = 1 << 1; // 编辑权限：0b0010
    const SHARE = 1 << 2;   // 分享权限：0b0100
    const DELETE = 1 << 3;  // 删除权限：0b1000
}
```

## 7. 关键代码片段

### 令牌服务初始化

```rust
// 令牌服务构造 ([`crates/board/src/services/token.rs:48`](crates/board/src/services/token.rs:48))
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
}
```

### 令牌签发核心逻辑

```rust
// 令牌签发 ([`crates/board/src/services/token.rs:61`](crates/board/src/services/token.rs:61))
pub fn issue(&self, room: &Room) -> Result<(String, RoomTokenClaims)> {
    // 1. 验证房间状态
    if room.is_expired() {
        return Err(anyhow!("room already expired"));
    }

    // 2. 计算过期时间
    let now = Utc::now();
    let mut exp = now + self.ttl;

    // 3. 应用房间过期时间约束
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

    // 4. 验证最小有效期
    if (exp - now).num_seconds() < MINIMUM_EXP_DELTA_SECONDS {
        return Err(anyhow!("token ttl too short after applying room expiry limit"));
    }

    // 5. 生成令牌
    let jti = Uuid::new_v4().to_string();
    let claims = RoomTokenClaims { /* ... */ };

    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(self.secret.as_bytes()),
    )?;

    Ok((token, claims))
}
```

### 权限验证函数

```rust
// 权限检查 ([`crates/board/src/handlers/content.rs:705`](crates/board/src/handlers/content.rs:705))
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

- **令牌生成测试**: 验证正常和异常情况下的令牌生成
- **令牌验证测试**: 测试有效、无效、过期令牌的处理
- **权限验证测试**: 验证各种权限组合的正确性
- **时间边界测试**: 测试过期时间边界的处理

### 集成测试建议

- **完整认证流程**: 从房间创建到令牌使用的完整流程
- **权限隔离测试**: 验证不同房间的令牌不能跨房间使用
- **并发令牌测试**: 多用户同时获取令牌的并发安全
- **令牌撤销测试**: 验证令牌撤销机制的有效性

### 安全测试

- **令牌伪造测试**: 尝试使用伪造签名访问系统
- **权限提升测试**: 尝试通过修改令牌内容提升权限
- **重放攻击测试**: 验证令牌重放攻击的防护
- **时序攻击测试**: 验证令牌验证的时序安全性

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

1. **令牌撤销机制不完整**: 当前缺乏完整的令牌撤销实现，需要添加黑名单机制
2. **密钥轮换支持缺失**: 没有支持运行时密钥轮换，需要重启服务

### P1 优先级

1. **令牌刷新机制**: 缺乏令牌自动刷新功能，用户体验有待改善
2. **审计日志不足**: 缺乏详细的认证事件审计日志

### P2 优先级

1. **多因素认证**: 可以考虑为敏感房间添加多因素认证
2. **令牌绑定**: 可以考虑将令牌与客户端 IP 或 User-Agent 绑定

## 10. 关联文档 / 代码位置

### 源码路径

- **令牌服务**:
  [`crates/board/src/services/token.rs`](crates/board/src/services/token.rs)
- **权限模型**:
  [`crates/board/src/models/room/permission.rs`](crates/board/src/models/room/permission.rs)
- **房间处理器**:
  [`crates/board/src/handlers/rooms.rs`](crates/board/src/handlers/rooms.rs)
- **内容处理器**:
  [`crates/board/src/handlers/content.rs`](crates/board/src/handlers/content.rs)

### 依赖配置

```toml
# JWT 依赖 ([`crates/board/Cargo.toml:57`](crates/board/Cargo.toml:57))
jsonwebtoken = { version = "10", features = ["use_pem", "aws_lc_rs"] }

# UUID 依赖 ([`crates/board/Cargo.toml:70`](crates/board/Cargo.toml:70))
uuid = { version = "1", features = ["v4", "serde"] }

# 时间处理依赖 ([`crates/board/Cargo.toml:55`](crates/board/Cargo.toml:55))
chrono = { version = "0.4", features = ["serde"] }
```

### 配置示例

```bash
# JWT 密钥配置（必须设置）
export JWT_SECRET="your-super-secret-key-here"

# 令牌默认有效期（分钟）
export JWT_DEFAULT_TTL_MINUTES=30

# 时钟容差（秒）
export JWT_LEEWAY_SECONDS=5

# 最小令牌有效期（秒）
export JWT_MIN_EXP_DELTA_SECONDS=5
```

### 安全配置建议

```bash
# 生产环境安全配置
export JWT_SECRET="$(openssl rand -base64 32)"
export JWT_DEFAULT_TTL_MINUTES=60
export JWT_LEEWAY_SECONDS=10

# 开发环境配置
export JWT_SECRET="dev-secret-key"
export JWT_DEFAULT_TTL_MINUTES=1440  # 24小时
```

### 相关文档

- [system-db.md](system-db.md) - 数据库系统和令牌持久化
- [system-crypto.md](system-crypto.md) - 加密系统和签名算法
- [model-room.md](model-room.md) - 房间模型和权限管理
- [handler-token.md](handler-token.md) - 令牌处理器详细说明
