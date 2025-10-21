# 加密系统 (Cryptography System)

## 1. 简介

Elizabeth 加密系统采用分层安全设计，涵盖 JWT
令牌签名、文件完整性校验、密码存储等多个安全层面。系统基于业界标准的加密算法库，使用
HMAC-SHA256 进行 JWT 签名，SHA2
系列算法进行文件完整性校验。当前实现采用"加密传输 +
明文存储"的安全模型，重点关注传输安全和数据完整性，为后续的端到端加密功能预留扩展空间。

主要交互方包括：

- 认证服务 (`crates/board/src/services/token.rs`) - JWT 签名和验证
- 文件处理系统 (`crates/board/src/handlers/content.rs`) - 文件完整性校验
- 密码管理模块 - 房间密码安全存储
- 安全配置系统 - 加密密钥和参数管理

## 2. 数据模型

### JWT 签名模型

```rust
// JWT 签名配置 ([`crates/board/src/services/token.rs:99`](crates/board/src/services/token.rs:99))
let token = jsonwebtoken::encode(
    &Header::new(Algorithm::HS256),  // HMAC-SHA256 算法
    &claims,
    &EncodingKey::from_secret(self.secret.as_bytes()), // 密钥
)
```

### 文件完整性校验模型

```rust
// 文件上传时的完整性检查
// 依赖：sha2 = "0.10" ([`crates/board/Cargo.toml:77`](crates/board/Cargo.toml:77))
// 当前实现：基于文件大小的简单校验
// 扩展计划：添加 SHA256 哈希校验
```

### 密码存储模型

```rust
// 房间密码字段 ([`crates/board/src/models/room/mod.rs:39`](crates/board/src/models/room/mod.rs:39))
pub struct Room {
    pub password: Option<String>,  // 当前为明文存储，计划升级为哈希存储
    // ... 其他字段
}
```

### 密钥管理模型

```rust
// 令牌服务密钥管理 ([`crates/board/src/services/token.rs:42`](crates/board/src/services/token.rs:42))
#[derive(Clone)]
pub struct RoomTokenService {
    secret: Arc<String>,    // JWT 签名密钥，通过环境变量管理
    ttl: Duration,
    leeway: i64,
}
```

## 3. 不变式 & 验证逻辑

### 安全规则

1. **密钥强度**: JWT 密钥必须至少 32 字节，使用高熵随机生成
2. **算法一致性**: 所有 JWT 令牌必须使用 HS256 算法签名和验证
3. **时间安全**: 令牌验证必须考虑时钟偏移，防止重放攻击
4. **完整性保证**: 文件上传后必须验证大小一致性
5. **密钥隔离**: 不同环境使用不同的签名密钥

### 验证逻辑

```rust
// JWT 验证配置 ([`crates/board/src/services/token.rs:110`](crates/board/src/services/token.rs:110))
pub fn decode(&self, token: &str) -> Result<RoomTokenClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = self.leeway as u64;  // 时钟容差
    let data = jsonwebtoken::decode::<RoomTokenClaims>(
        token,
        &DecodingKey::from_secret(self.secret.as_bytes()),
        &validation,
    )
    .context("invalid token")?;

    Ok(data.claims)
}
```

### 安全约束

- **密钥轮换**: 支持通过重启服务进行密钥轮换
- **令牌唯一性**: 每个 JWT 都有唯一的 JTI 标识符
- **权限绑定**: 令牌权限与房间权限严格绑定，防止权限提升

## 4. 持久化 & 索引

### 密钥存储策略

- **环境变量**: JWT 密钥通过 `JWT_SECRET` 环境变量管理
- **内存缓存**: 密钥在服务启动时加载到内存，避免重复读取
- **安全清除**: 服务关闭时内存中的密钥自动清除

### 加密数据索引

```sql
-- 令牌表索引 ([`crates/board/migrations/005_create_room_tokens_table.sql`](crates/board/migrations/005_create_room_tokens_table.sql:12))
CREATE INDEX IF NOT EXISTS idx_room_tokens_expires_at ON room_tokens(expires_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_room_tokens_jti ON room_tokens(jti);
```

### 密码存储索引

```sql
-- 房间表密码字段 ([`crates/board/migrations/001_create_rooms_table.sql`](crates/board/migrations/001_create_rooms_table.sql:5))
-- 当前为明文存储，计划升级为哈希存储
password TEXT,  -- TODO: 升级为 password_hash TEXT
```

## 5. API/Handlers

### 加密相关 API

- **JWT 生成**: `RoomTokenService::issue()` - 生成签名令牌
- **JWT 验证**: `RoomTokenService::decode()` - 验证令牌签名
- **文件完整性**: 文件上传时的大小验证
- **密码验证**: 房间进入时的密码检查

### 安全中间件

```rust
// 令牌验证中间件 ([`crates/board/src/handlers/mod.rs`](crates/board/src/handlers/mod.rs))
pub async fn verify_room_token(
    app_state: Arc<AppState>,
    room_name: &str,
    token: &str,
) -> Result<VerifiedToken, HttpResponse>
```

### 文件完整性检查

```rust
// 文件大小验证 ([`crates/board/src/handlers/content.rs:415`](crates/board/src/handlers/content.rs:415))
if size != expected.size {
    fs::remove_file(&file_path).await.ok();
    for temp in &staged {
        fs::remove_file(&temp.path).await.ok();
    }
    return Err(HttpResponse::BadRequest().message(format!("File size mismatch for {file_name}")));
}
```

## 6. JWT 与权限

### JWT 签名流程

1. **准备 Claims**: 构建包含房间信息和权限的声明
2. **设置 Header**: 使用 HS256 算法标识
3. **生成签名**: 使用 HMAC-SHA256 算法和密钥签名
4. **组合令牌**: 将 header、payload、signature 组合成 JWT

### 权限加密保护

```rust
// 权限信息在 JWT 中的加密传输 ([`crates/board/src/services/token.rs:88`](crates/board/src/services/token.rs:88))
let claims = RoomTokenClaims {
    sub: format!("room:{}", room.id.unwrap_or_default()),
    room_id: room.id.ok_or_else(|| anyhow!("room id missing"))?,
    room_name: room.slug.clone(),
    permission: room.permission.bits(),  // 权限位掩码，防篡改
    max_size: room.max_size,
    exp: exp.timestamp(),
    iat: now.timestamp(),
    jti,
};
```

### 密钥管理最佳实践

- **生产环境**: 使用 32 字节以上的高熵随机密钥
- **开发环境**: 使用固定密钥便于调试
- **测试环境**: 每个测试套件使用独立密钥
- **密钥轮换**: 通过环境变量更新，重启服务生效

## 7. 关键代码片段

### JWT 签名核心实现

```rust
// JWT 令牌签名 ([`crates/board/src/services/token.rs:99`](crates/board/src/services/token.rs:99))
let token = jsonwebtoken::encode(
    &Header::new(Algorithm::HS256),
    &claims,
    &EncodingKey::from_secret(self.secret.as_bytes()),
)
.context("failed to sign room token")?;
```

### JWT 验证核心实现

```rust
// JWT 令牌验证 ([`crates/board/src/services/token.rs:109`](crates/board/src/services/token.rs:109))
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

### 文件完整性验证

```rust
// 文件大小一致性检查 ([`crates/board/src/handlers/content.rs:401`](crates/board/src/handlers/content.rs:401))
let mut size: i64 = 0;
while let Some(chunk) = field.next().await {
    let chunk = chunk.map_err(|e| {
        HttpResponse::BadRequest().message(format!("Read upload chunk failed: {e}"))
    })?;
    size += chunk.len() as i64;
    temp_file.write_all(&chunk).await.map_err(|e| {
        HttpResponse::InternalServerError().message(format!("Write file failed: {e}"))
    })?;
}

// 验证大小一致性
if size != expected.size {
    fs::remove_file(&file_path).await.ok();
    return Err(HttpResponse::BadRequest().message(format!("File size mismatch for {file_name}")));
}
```

### 密钥初始化

```rust
// 令牌服务初始化 ([`crates/board/src/services/token.rs:48`](crates/board/src/services/token.rs:48))
impl RoomTokenService {
    pub fn new(secret: Arc<String>) -> Self {
        Self::with_ttl(secret, Duration::minutes(DEFAULT_TOKEN_TTL_MINUTES))
    }

    pub fn with_ttl(secret: Arc<String>, ttl: Duration) -> Self {
        Self {
            secret,
            ttl,
            leeway: DEFAULT_LEEWAY_SECONDS,  // 5 秒时钟容差
        }
    }
}
```

## 8. 测试要点

### 单元测试建议

- **JWT 签名验证测试**: 测试正常和异常情况下的签名生成和验证
- **密钥强度测试**: 验证不同长度密钥的安全性
- **时间攻击测试**: 测试令牌验证的时序安全性
- **完整性校验测试**: 测试文件大小验证的正确性

### 集成测试建议

- **端到端加密测试**: 从令牌生成到验证的完整流程
- **密钥轮换测试**: 验证密钥更换后的系统行为
- **并发安全测试**: 多线程环境下的加密操作安全性
- **性能基准测试**: 加密操作的性能基准测试

### 安全测试

- **令牌伪造测试**: 尝试使用错误密钥伪造令牌
- **权限篡改测试**: 尝试修改令牌中的权限信息
- **重放攻击测试**: 验证令牌重放的防护机制
- **时序攻击测试**: 验证密码比较的时序安全性

## 9. 已知问题 / TODO / 改进建议

### P0 优先级

1. **密码明文存储**: 房间密码当前为明文存储，需要升级为 bcrypt/argon2 哈希
2. **文件完整性校验不足**: 当前仅验证文件大小，需要添加 SHA256 哈希校验

### P1 优先级

1. **密钥轮换机制**: 缺乏运行时密钥轮换机制，需要重启服务
2. **加密密钥管理**: 缺乏密钥版本管理和回滚机制

### P2 优先级

1. **端到端加密**: 可以考虑添加文件内容的端到端加密
2. **硬件安全模块**: 可以考虑集成 HSM 进行密钥管理

### 未来扩展计划

```rust
// 计划中的密码哈希升级
pub struct Room {
    pub password_hash: Option<String>,  // 升级为哈希存储
    pub password_salt: Option<String>,  // 盐值
    pub password_algorithm: Option<String>, // 哈希算法标识
}

// 计划中的文件完整性校验
pub struct RoomContent {
    pub file_hash: Option<String>,  // SHA256 哈希
    pub file_size: i64,            // 文件大小
    // ... 其他字段
}
```

## 10. 关联文档 / 代码位置

### 源码路径

- **令牌服务**:
  [`crates/board/src/services/token.rs`](crates/board/src/services/token.rs)
- **房间模型**:
  [`crates/board/src/models/room/mod.rs`](crates/board/src/models/room/mod.rs)
- **内容处理器**:
  [`crates/board/src/handlers/content.rs`](crates/board/src/handlers/content.rs)
- **认证处理器**:
  [`crates/board/src/handlers/rooms.rs`](crates/board/src/handlers/rooms.rs)

### 依赖配置

```toml
# JWT 加密依赖 ([`crates/board/Cargo.toml:57`](crates/board/Cargo.toml:57))
jsonwebtoken = { version = "10", features = ["use_pem", "aws_lc_rs"] }

# 哈希算法依赖 ([`crates/board/Cargo.toml:77`](crates/board/Cargo.toml:77))
sha2 = "0.10"

# 密码学依赖 ([`crates/board/Cargo.toml:57`](crates/board/Cargo.toml:57))
# aws_lc_rs 提供现代密码学算法实现
```

### 安全配置示例

```bash
# 生产环境安全配置
export JWT_SECRET="$(openssl rand -base64 32)"
export JWT_LEEWAY_SECONDS=10
export JWT_DEFAULT_TTL_MINUTES=60

# 开发环境配置
export JWT_SECRET="dev-secret-key-change-in-production"
export JWT_LEEWAY_SECONDS=5
export JWT_DEFAULT_TTL_MINUTES=1440

# 测试环境配置
export JWT_SECRET="test-secret-key"
export JWT_LEEWAY_SECONDS=0
export JWT_DEFAULT_TTL_MINUTES=30
```

### 密钥生成脚本

```bash
#!/bin/bash
# 生成安全的 JWT 密钥
generate_jwt_secret() {
    if command -v openssl >/dev/null 2>&1; then
        openssl rand -base64 32
    elif command -v head >/dev/null 2>&1 && command -v base64 >/dev/null 2>&1; then
        head -c 32 /dev/urandom | base64
    else
        echo "Error: openssl or head/base64 not available" >&2
        exit 1
    fi
}

# 使用示例
JWT_SECRET=$(generate_jwt_secret)
echo "Generated JWT Secret: $JWT_SECRET"
```

### 安全检查清单

- [ ] JWT 密钥长度 >= 32 字节
- [ ] 生产环境使用高熵随机密钥
- [ ] 令牌验证包含时钟容差
- [ ] 文件上传包含完整性校验
- [ ] 密码存储使用强哈希算法（计划中）
- [ ] 定期轮换签名密钥
- [ ] 启用安全传输协议 (HTTPS)

### 相关文档

- [system-auth.md](system-auth.md) - 认证系统和令牌管理
- [system-storage.md](system-storage.md) - 存储系统和文件安全
- [system-db.md](system-db.md) - 数据库系统和数据保护
- [security-best-practices.md](security-best-practices.md) -
  安全最佳实践（待创建）
