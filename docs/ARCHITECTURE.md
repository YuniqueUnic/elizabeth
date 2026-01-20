# Elizabeth 项目架构文档

本文档详细描述 Elizabeth 项目的代码架构、模块组织、数据流程和核心逻辑实现。

## 目录

- [1. 项目总览](#1-项目总览)
- [2. 后端架构深度剖析](#2-后端架构深度剖析)
- [3. 核心业务流程](#3-核心业务流程)
- [4. 数据模型设计](#4-数据模型设计)
- [5. 安全性设计](#5-安全性设计)
- [6. 性能优化](#6-性能优化)
- [7. 扩展性设计](#7-扩展性设计)

---

## 1. 项目总览

### 1.1 技术栈全景图

```
┌─────────────────────────────────────────────────────────────────┐
│                        Elizabeth 技术栈                          │
└─────────────────────────────────────────────────────────────────┘
                                │
                ┌───────────────┴────────────────┐
                │                                │
        ┌───────▼────────┐              ┌───────▼────────┐
        │   前端技术栈    │              │   后端技术栈    │
        └────────────────┘              └────────────────┘
                │                                │
    ┌───────────┼───────────┐        ┌──────────┼──────────┐
    │           │           │        │          │          │
┌───▼───┐  ┌───▼───┐  ┌───▼───┐ ┌──▼──┐  ┌───▼───┐  ┌───▼───┐
│Next.js│  │shadcn/│  │Zustand│ │Axum │  │SQLx   │  │Tokio  │
│  16   │  │  ui   │  │       │ │0.8.6│  │ 0.8   │  │       │
└───────┘  └───────┘  └───────┘ └─────┘  └───────┘  └───────┘
```

### 1.2 Crates 组织结构

Elizabeth 后端采用 Cargo Workspace 组织，包含以下 crates：

```
elizabeth/
├── crates/
│   ├── board/              # 主应用程序（二进制 crate）
│   ├── board-protocol/     # 共享协议和模型（库 crate）
│   ├── configrs/           # 配置管理库（库 crate）
│   └── logrs/              # 日志管理库（库 crate）
```

#### 1.2.1 board (主应用)

**职责**: 提供 HTTP API 服务、WebSocket 服务、业务逻辑实现

**依赖关系**:

```rust
[dependencies]
board_protocol = { workspace = true }    // 数据模型和 DTO
configrs = { workspace = true }          // 配置管理
logrs = { workspace = true }             // 日志记录

axum = "0.8.6"                          // Web 框架
sqlx = "0.8"                            // 数据库访问
tokio = { features = ["full"] }          // 异步运行时
utoipa = "5.3"                          // OpenAPI 文档
serde = "1.0"                           // 序列化
jsonwebtoken = "9.3"                    // JWT 认证
```

**模块组织**:

```
src/
├── main.rs              # 程序入口
├── lib.rs               # 库入口，导出公共模块
├── cmd/                 # 命令行处理
├── config.rs            # 配置结构定义
├── constants/           # 常量定义
├── db.rs                # 数据库连接池
├── errors.rs            # 错误类型定义
├── handlers/            # HTTP 请求处理器
│   ├── rooms.rs         # 房间管理 API
│   ├── content.rs       # 内容管理 API
│   ├── chunked_upload.rs # 分块上传 API
│   ├── auth.rs          # 认证 API
│   ├── refresh_token.rs # 刷新令牌 API
│   └── admin.rs         # 管理员 API
├── repository/          # 数据访问层
│   ├── room_repository.rs
│   ├── content_repository.rs
│   ├── token_repository.rs
│   ├── refresh_token_repository.rs
│   ├── chunk_upload_repository.rs
│   └── upload_reservation_repository.rs
├── route/               # 路由定义
│   ├── room.rs          # 房间路由
│   ├── auth.rs          # 认证路由
│   ├── ws.rs            # WebSocket 路由
│   ├── admin.rs         # 管理路由
│   └── status.rs        # 状态路由
├── services/            # 业务逻辑服务
│   ├── auth_service.rs      # 认证服务
│   ├── token_service.rs     # Token 管理
│   └── room_gc_service.rs   # 房间垃圾回收
├── middleware/          # 中间件
│   ├── cors.rs          # CORS 配置
│   ├── request_id.rs    # 请求 ID 追踪
│   └── security.rs      # 安全头设置
├── websocket/           # WebSocket 模块
│   ├── server.rs        # WebSocket 服务器
│   ├── handler.rs       # 消息处理器
│   ├── connection.rs    # 连接管理
│   └── types.rs         # WebSocket 类型
├── validation/          # 数据验证
├── storage/             # 存储抽象
├── state.rs             # 应用状态
└── tests/               # 集成测试
```

#### 1.2.2 board-protocol (协议定义)

**职责**: 定义共享的数据模型、DTO、常量，前后端类型一致性

**特性**:

```rust
[features]
default = []
typescript-export = ["dep:schemars", "dep:ts-rs"]  // TypeScript 类型导出
```

**模块组织**:

```
src/
├── lib.rs               # 库入口
├── models/              # 数据模型
│   ├── room/            # 房间相关模型
│   │   ├── mod.rs       # Room 核心模型
│   │   ├── content.rs   # RoomContent 模型
│   │   ├── token.rs     # RoomToken 模型
│   │   ├── refresh_token.rs    # RefreshToken 模型
│   │   ├── chunk_upload.rs     # ChunkUpload 模型
│   │   ├── upload_reservation.rs  # UploadReservation 模型
│   │   └── permission.rs       # Permission 模型
│   └── permission.rs    # 权限枚举
├── dto/                 # 数据传输对象
│   ├── room.rs          # 房间 DTO
│   ├── content.rs       # 内容 DTO
│   ├── token.rs         # Token DTO
│   └── upload.rs        # 上传 DTO
└── constants/           # 常量定义
    ├── room.rs          # 房间相关常量
    ├── upload.rs        # 上传相关常量
    └── storage.rs       # 存储相关常量
```

**TypeScript 类型生成**:

通过 `ts-rs` crate，Rust 结构体可以自动生成 TypeScript 类型：

```rust
// Rust 定义
#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/types/generated/")]
pub struct Room {
    pub id: Option<i64>,
    pub name: String,
    pub slug: String,
    // ...
}

// 自动生成 TypeScript
// web/types/generated/Room.ts
export interface Room {
    id: number | null;
    name: string;
    slug: string;
    // ...
}
```

#### 1.2.3 configrs (配置管理)

**职责**: 提供分层配置加载、环境变量覆盖、配置文件管理

**配置加载流程**:

```
┌──────────────┐
│ 默认配置     │  1. 代码中的 Default trait
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ 配置文件     │  2. ~/.config/elizabeth/config.yaml
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ 环境变量     │  3. ELIZABETH__APP__* 前缀
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ 最终配置     │
└──────────────┘
```

补充：为了运维与 Docker 部署更方便，程序还支持一组“快捷环境变量”（如
`DATABASE_URL`、`DB_MAX_CONNECTIONS`、`DB_MIN_CONNECTIONS`、`JWT_SECRET`、`LOG_LEVEL`
等），会在启动时覆盖 configrs 合并后的配置（实现见
`crates/board/src/init/cfg_service.rs`）。

**配置结构**:

```rust
pub struct AppConfig {
    pub server: ServerConfig,        // 服务器配置
    pub logging: LoggingConfig,      // 日志配置
    pub database: DatabaseConfig,    // 数据库配置
    pub storage: StorageConfig,      // 存储配置
    pub jwt: JwtConfig,              // JWT 配置
    pub room: RoomConfig,            // 房间配置
    pub middleware: MiddlewareConfig,// 中间件配置
}
```

#### 1.2.4 logrs (日志管理)

**职责**: 统一日志接口、结构化日志、日志级别控制

---

## 2. 后端架构深度剖析

### 2.1 分层架构详解

```
┌─────────────────────────────────────────────────────────────────┐
│                       请求处理流程                               │
└─────────────────────────────────────────────────────────────────┘

HTTP Request
     │
     ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Middleware Layer (中间件层)                   │
│  - CORS                    跨域处理                              │
│  - RequestId               请求追踪                              │
│  - Security Headers        安全头设置                            │
│  - Compression             响应压缩                              │
│  - Rate Limiting           速率限制                              │
│  - Request Tracing         请求追踪                              │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Route Layer (路由层)                        │
│  职责：                                                          │
│  - 定义 API 端点路径                                              │
│  - 绑定 HTTP 方法到 Handler                                       │
│  - 配置 OpenAPI 文档                                              │
│                                                                  │
│  实现：                                                          │
│  pub fn api_router(state: AppState) -> OpenApiRouter {          │
│      OpenApiRouter::new()                                        │
│          .routes(routes!(handlers::rooms::create))               │
│          .routes(routes!(handlers::rooms::find))                 │
│          .with_state(state)                                      │
│  }                                                               │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Handler Layer (处理层)                        │
│  职责：                                                          │
│  - 解析 HTTP 请求参数（Path, Query, Body, Header）               │
│  - 执行数据验证                                                   │
│  - 调用 Service 层业务逻辑                                         │
│  - 构建 HTTP 响应                                                 │
│  - 错误处理和转换                                                 │
│                                                                  │
│  示例：                                                          │
│  pub async fn create(                                            │
│      Path(name): Path<String>,                                   │
│      Query(params): Query<CreateRoomParams>,                     │
│      State(app_state): State<Arc<AppState>>,                     │
│  ) -> HandlerResult<Room> {                                      │
│      // 1. 验证输入                                               │
│      RoomNameValidator::validate(&name)?;                        │
│      // 2. 调用 Repository                                       │
│      let repository = RoomRepository::new(app_state.db_pool);    │
│      let room = repository.create(&room).await?;                 │
│      // 3. 返回响应                                               │
│      Ok(Json(room))                                              │
│  }                                                               │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Service Layer (服务层)                         │
│  职责：                                                          │
│  - 封装复杂业务逻辑                                               │
│  - 跨 Repository 的事务协调                                       │
│  - Token 生成和验证                                               │
│  - 权限检查                                                       │
│  - 房间清理和垃圾回收                                             │
│                                                                  │
│  核心服务：                                                       │
│  - AuthService: 认证和授权                                        │
│  - TokenService: JWT Token 管理                                  │
│  - RoomGcService: 房间自动清理                                    │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Repository Layer (仓库层)                        │
│  职责：                                                          │
│  - 数据库 CRUD 操作                                               │
│  - SQL 查询封装                                                   │
│  - 事务管理                                                       │
│  - 数据库错误处理                                                 │
│                                                                  │
│  Repository Pattern 实现：                                        │
│  pub struct RoomRepository {                                     │
│      pool: PgPool,                                               │
│  }                                                               │
│                                                                  │
│  impl RoomRepository {                                           │
│      pub async fn create(&self, room: &Room) -> Result<Room> {  │
│          let mut tx = self.pool.begin().await?;                  │
│          // SQL 插入操作                                          │
│          let result = sqlx::query_as::<_, Room>(...)             │
│              .fetch_one(&mut *tx).await?;                        │
│          tx.commit().await?;                                     │
│          Ok(result)                                              │
│      }                                                           │
│  }                                                               │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Database Layer (数据库层)                     │
│  - SQLite (默认)                                                 │
│  - PostgreSQL (可选)                                             │
│  - 连接池管理 (SQLx Pool)                                         │
│  - 自动迁移 (sqlx migrate)                                        │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 核心模块详解

#### 2.2.1 Room (房间模块)

**数据结构**:

```rust
pub struct Room {
    pub id: Option<i64>,              // 数据库主键
    pub name: String,                  // 显示名称
    pub slug: String,                  // URL 标识符（唯一）
    pub password: Option<String>,      // 加密后的密码
    pub status: i32,                   // 状态：0=活跃，1=已满，2=已过期
    pub max_size: i64,                 // 最大容量（字节）
    pub current_size: i64,             // 当前使用（字节）
    pub max_times_entered: i64,        // 最大进入次数
    pub current_times_entered: i64,    // 当前进入次数
    pub expire_at: Option<NaiveDateTime>, // 过期时间
    pub created_at: NaiveDateTime,     // 创建时间
    pub updated_at: NaiveDateTime,     // 更新时间
    pub permission: RoomPermission,    // 权限位标志
}
```

**核心方法**:

```rust
impl Room {
    // 创建新房间
    pub fn new(name: String, password: Option<String>) -> Self {
        let slug = Self::generate_slug(&name);
        let hashed_password = password.map(|p| hash_password(&p));
        Self {
            id: None,
            name,
            slug,
            password: hashed_password,
            status: 0,
            max_size: DEFAULT_MAX_ROOM_CONTENT_SIZE,
            current_size: 0,
            max_times_entered: DEFAULT_MAX_TIMES_ENTER_ROOM,
            current_times_entered: 0,
            expire_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            permission: RoomPermission::all(),
        }
    }

    // 检查房间是否可以进入
    pub fn can_enter(&self) -> bool {
        self.status == 0 &&
        self.current_times_entered < self.max_times_entered &&
        !self.is_expired()
    }

    // 检查房间是否已过期
    pub fn is_expired(&self) -> bool {
        if let Some(expire_at) = self.expire_at {
            Utc::now().naive_utc() > expire_at
        } else {
            false
        }
    }

    // 检查是否可以添加内容
    pub fn can_add_content(&self, size: i64) -> bool {
        self.current_size + size <= self.max_size &&
        self.permission.can_edit()
    }

    // 验证密码
    pub fn verify_password(&self, password: &str) -> bool {
        if let Some(ref hashed) = self.password {
            verify_password(password, hashed)
        } else {
            true  // 无密码房间
        }
    }

    // 生成 slug（URL 标识符）
    fn generate_slug(name: &str) -> String {
        format!("{}-{}",
            name.to_lowercase().replace(' ', '-'),
            Uuid::new_v4().to_string()[..8].to_string()
        )
    }
}
```

**Repository 实现**:

```rust
pub struct RoomRepository {
    pool: Arc<PgPool>,
}

impl RoomRepository {
    // 创建房间
    pub async fn create(&self, room: &Room) -> Result<Room> {
        let mut tx = self.pool.begin().await?;

        // 插入房间记录
        let inserted_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO rooms (
                name, slug, password, status, max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id
            "#
        )
        .bind(&room.name)
        .bind(&room.slug)
        .bind(&room.password)
        .bind(room.status)
        .bind(room.max_size)
        .bind(room.current_size)
        .bind(room.max_times_entered)
        .bind(room.current_times_entered)
        .bind(room.expire_at)
        .bind(room.created_at)
        .bind(room.updated_at)
        .bind(i64::from(room.permission.bits()))
        .fetch_one(&mut *tx)
        .await?;

        // 查询完整记录
        let created_room = self.fetch_room_by_id(&mut *tx, inserted_id).await?;

        tx.commit().await?;
        Ok(created_room)
    }

    // 根据 slug 查找房间
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        let room = sqlx::query_as::<_, Room>(
            r#"
            SELECT * FROM rooms WHERE slug = $1
            "#
        )
        .bind(name)
        .fetch_optional(&*self.pool)
        .await?;

        // 如果房间已过期，重置状态
        if let Some(room) = room {
            self.reset_if_expired(room).await
        } else {
            Ok(None)
        }
    }

    // 检查并重置过期房间
    async fn reset_if_expired(&self, mut room: Room) -> Result<Option<Room>> {
        if room.is_expired() {
            room.status = 2;  // 标记为已过期
            self.update(&room).await?;
            Ok(None)
        } else {
            Ok(Some(room))
        }
    }

    // 更新房间
    pub async fn update(&self, room: &Room) -> Result<Room> {
        let id = room.id.ok_or_else(|| anyhow!("Room ID is required"))?;

        sqlx::query(
            r#"
            UPDATE rooms SET
                name = $1, password = $2, status = $3,
                current_size = $4, current_times_entered = $5,
                expire_at = $6, updated_at = $7, permission = $8
            WHERE id = $9
            "#
        )
        .bind(&room.name)
        .bind(&room.password)
        .bind(room.status)
        .bind(room.current_size)
        .bind(room.current_times_entered)
        .bind(room.expire_at)
        .bind(Utc::now().naive_utc())
        .bind(i64::from(room.permission.bits()))
        .bind(id)
        .execute(&*self.pool)
        .await?;

        self.find_by_id(id).await?
            .ok_or_else(|| anyhow!("Room not found after update"))
    }
}
```

#### 2.2.2 RoomContent (房间内容模块)

**内容类型设计**:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    File = 0,    // 文件类型
    Text = 1,    // 文本类型
    Url = 2,     // URL 类型
}

pub struct RoomContent {
    pub id: Option<i64>,
    pub room_id: i64,
    pub content_type: ContentType,

    // 不同类型使用不同字段
    pub display_name: String,        // 显示名称
    pub file_path: Option<String>,   // 文件路径（File 类型）
    pub text_content: Option<String>,// 文本内容（Text 类型）
    pub url: Option<String>,         // URL（Url 类型）

    pub mime_type: Option<String>,   // MIME 类型
    pub size: i64,                   // 大小（字节）
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
```

**文件存储策略**:

```
storage/rooms/
├── {room_slug}/              # 房间目录
│   ├── {uuid}-{filename}     # 实际文件（UUID 防止冲突）
│   ├── {uuid}-{filename}
│   └── ...
└── temp/                     # 临时上传目录
    └── {upload_token}/       # 分块上传临时目录
        ├── chunk_0
        ├── chunk_1
        └── ...
```

**文件命名规则**:

```rust
pub fn generate_file_path(room_slug: &str, original_name: &str) -> String {
    let uuid = Uuid::new_v4();
    let sanitized_name = sanitize_filename::sanitize(original_name);
    format!("{}/{}-{}", room_slug, uuid, sanitized_name)
}
```

#### 2.2.3 Token (认证模块)

**JWT Claims 结构**:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct RoomTokenClaims {
    pub jti: String,              // JWT ID（唯一标识符）
    pub room_id: i64,             // 房间 ID
    pub room_name: String,        // 房间名称
    pub permission: i64,          // 权限位标志
    pub token_type: String,       // 令牌类型：access/refresh
    pub iat: i64,                 // 签发时间
    pub exp: i64,                 // 过期时间
}

impl RoomTokenClaims {
    // 检查是否过期
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    // 检查是否即将过期（5 分钟内）
    pub fn is_expiring_soon(&self) -> bool {
        let remaining = self.exp - Utc::now().timestamp();
        remaining < 300  // 5 minutes
    }

    // 剩余有效时间（秒）
    pub fn remaining_seconds(&self) -> i64 {
        self.exp - Utc::now().timestamp()
    }

    // 转换为权限枚举
    pub fn as_permission(&self) -> RoomPermission {
        RoomPermission::from_bits_truncate(self.permission as u8)
    }

    // 是否为访问令牌
    pub fn is_access_token(&self) -> bool {
        self.token_type == "access"
    }

    // 是否为刷新令牌
    pub fn is_refresh_token(&self) -> bool {
        self.token_type == "refresh"
    }
}
```

**Token 生成流程**:

```rust
pub struct TokenService {
    secret: String,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

impl TokenService {
    // 签发访问令牌
    pub fn issue(&self, room: &Room) -> Result<(String, RoomTokenClaims)> {
        let jti = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        let claims = RoomTokenClaims {
            jti: jti.clone(),
            room_id: room.id.unwrap(),
            room_name: room.slug.clone(),
            permission: i64::from(room.permission.bits()),
            token_type: "access".to_string(),
            iat: now,
            exp: now + self.access_ttl.num_seconds(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes())
        )?;

        Ok((token, claims))
    }

    // 签发刷新令牌
    pub fn issue_refresh_token(
        &self,
        room: &Room,
        access_token_jti: &str
    ) -> Result<(String, RoomTokenClaims)> {
        let jti = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        let claims = RoomTokenClaims {
            jti: jti.clone(),
            room_id: room.id.unwrap(),
            room_name: room.slug.clone(),
            permission: i64::from(room.permission.bits()),
            token_type: "refresh".to_string(),
            iat: now,
            exp: now + self.refresh_ttl.num_seconds(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes())
        )?;

        Ok((token, claims))
    }

    // 解码和验证令牌
    pub fn decode(&self, token: &str) -> Result<RoomTokenClaims> {
        let token_data = decode::<RoomTokenClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default()
        )?;

        Ok(token_data.claims)
    }
}
```

**Token 黑名单机制**:

```rust
// 用于存储被撤销的 Token
pub struct TokenBlacklistEntry {
    pub jti: String,
    pub expires_at: NaiveDateTime,
}

// 黑名单 Repository
pub struct TokenBlacklistRepository {
    pool: Arc<PgPool>,
}

impl TokenBlacklistRepository {
    // 添加到黑名单
    pub async fn add(&self, jti: &str, expires_at: NaiveDateTime) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO token_blacklist (jti, expires_at)
            VALUES ($1, $2)
            ON CONFLICT (jti) DO NOTHING
            "#
        )
        .bind(jti)
        .bind(expires_at)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    // 检查是否在黑名单中
    pub async fn is_blacklisted(&self, jti: &str) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM token_blacklist
            WHERE jti = $1 AND expires_at > NOW()
            "#
        )
        .bind(jti)
        .fetch_one(&*self.pool)
        .await?;

        Ok(count > 0)
    }

    // 清理过期的黑名单记录
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM token_blacklist WHERE expires_at <= NOW()
            "#
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
```

#### 2.2.4 Permission (权限模块)

**位标志设计**:

```rust
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RoomPermission: u8 {
        const PREVIEW = 0b0000_0001;  // 预览权限（查看）
        const DOWNLOAD = 0b0000_0010; // 下载权限
        const EDIT = 0b0000_0100;     // 编辑权限（上传、修改、删除）
    }
}

impl RoomPermission {
    // 检查是否有预览权限
    pub fn can_preview(&self) -> bool {
        self.contains(Self::PREVIEW)
    }

    // 检查是否有下载权限
    pub fn can_download(&self) -> bool {
        self.contains(Self::DOWNLOAD)
    }

    // 检查是否有编辑权限
    pub fn can_edit(&self) -> bool {
        self.contains(Self::EDIT)
    }

    // 所有权限
    pub fn all() -> Self {
        Self::PREVIEW | Self::DOWNLOAD | Self::EDIT
    }

    // 只读权限
    pub fn readonly() -> Self {
        Self::PREVIEW | Self::DOWNLOAD
    }

    // 仅预览
    pub fn preview_only() -> Self {
        Self::PREVIEW
    }
}
```

**权限验证**:

```rust
// 在 Handler 中验证权限
pub async fn upload_contents(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<TokenQuery>,
    multipart: Multipart,
) -> HandlerResult<UploadContentResponse> {
    // 1. 验证 Token
    let verified = verify_room_token(app_state.clone(), &name, &query.token).await?;

    // 2. 检查权限
    ensure_permission(
        &verified.claims,
        verified.room.permission.can_edit(),
        ContentPermission::Edit,
    )?;

    // 3. 执行上传逻辑
    // ...
}

// 权限检查函数
fn ensure_permission(
    claims: &RoomTokenClaims,
    room_has_permission: bool,
    required: ContentPermission,
) -> Result<()> {
    if !room_has_permission {
        return Err(AppError::permission_denied("Room does not have required permission"));
    }

    let user_permission = claims.as_permission();
    match required {
        ContentPermission::Preview => {
            if !user_permission.can_preview() {
                return Err(AppError::permission_denied("Preview permission required"));
            }
        }
        ContentPermission::Download => {
            if !user_permission.can_download() {
                return Err(AppError::permission_denied("Download permission required"));
            }
        }
        ContentPermission::Edit => {
            if !user_permission.can_edit() {
                return Err(AppError::permission_denied("Edit permission required"));
            }
        }
    }

    Ok(())
}
```

---

## 3. 核心业务流程

### 3.1 房间创建流程

```
用户请求: POST /api/v1/rooms/{name}?password=xxx

┌──────────────────────────────────────────────────────────────┐
│ Step 1: Handler 接收请求                                      │
│                                                               │
│ pub async fn create(                                          │
│     Path(name): Path<String>,                                 │
│     Query(params): Query<CreateRoomParams>,                   │
│     State(app_state): State<Arc<AppState>>,                   │
│ ) -> HandlerResult<Room>                                      │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 2: 数据验证                                              │
│                                                               │
│ RoomNameValidator::validate(&name)?;                          │
│ - 长度检查：1-64 字符                                         │
│ - 字符检查：允许字母、数字、中文、-_空格                      │
│                                                               │
│ if let Some(ref password) = params.password {                 │
│     PasswordValidator::validate_room_password(password)?;     │
│     - 长度检查：6-128 字符                                     │
│ }                                                             │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 3: 检查房间是否存在                                      │
│                                                               │
│ let repository = RoomRepository::new(app_state.db_pool);      │
│ if repository.exists(&name).await? {                          │
│     return Err(AppError::conflict("Room already exists"));    │
│ }                                                             │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 4: 创建 Room 对象                                        │
│                                                               │
│ let mut room = Room::new(name.clone(), params.password);     │
│ - slug = "{name}-{uuid}"                                      │
│ - password = hash(password) if provided                       │
│ - max_size = DEFAULT_MAX_ROOM_CONTENT_SIZE (50MB)            │
│ - max_times_entered = DEFAULT_MAX_TIMES_ENTER_ROOM (100)     │
│ - permission = RoomPermission::all()                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 5: 应用配置默认值                                        │
│                                                               │
│ apply_room_defaults(&mut room, &app_state);                  │
│ - 从 AppConfig 读取最大大小和进入次数                          │
│ - 如果设置了默认过期时间，应用到房间                           │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 6: 写入数据库                                            │
│                                                               │
│ let created_room = repository.create(&room).await?;           │
│                                                               │
│ 数据库操作：                                                  │
│ BEGIN TRANSACTION                                             │
│   INSERT INTO rooms (...) VALUES (...)                        │
│   SELECT * FROM rooms WHERE id = LAST_INSERT_ID()             │
│ COMMIT                                                        │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 7: 返回响应                                              │
│                                                               │
│ Ok(Json(created_room))                                        │
│                                                               │
│ 响应示例：                                                    │
│ {                                                             │
│   "id": 1,                                                    │
│   "name": "myroom",                                           │
│   "slug": "myroom-a1b2c3d4",                                  │
│   "status": 0,                                                │
│   "max_size": 52428800,                                       │
│   "current_size": 0,                                          │
│   "max_times_entered": 100,                                   │
│   "current_times_entered": 0,                                 │
│   "expire_at": null,                                          │
│   "created_at": "2024-01-20T10:00:00",                        │
│   "permission": 7                                             │
│ }                                                             │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 Token 签发与验证流程

```
用户请求: POST /api/v1/rooms/{name}/tokens
Body: { "password": "xxx", "with_refresh_token": true }

┌──────────────────────────────────────────────────────────────┐
│ Step 1: Handler 接收请求                                      │
│                                                               │
│ pub async fn issue_token(                                     │
│     Path(name): Path<String>,                                 │
│     State(app_state): State<Arc<AppState>>,                   │
│     Json(payload): Json<IssueTokenRequest>,                   │
│ ) -> HandlerResult<IssueTokenResponse>                        │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 2: 查找房间                                              │
│                                                               │
│ let repository = RoomRepository::new(app_state.db_pool);      │
│ let mut room = repository.find_by_name(&name).await?          │
│     .ok_or_else(|| AppError::not_found("Room not found"))?;   │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 3: 验证密码                                              │
│                                                               │
│ if let Some(ref password) = payload.password {                │
│     if !room.verify_password(password) {                      │
│         return Err(AppError::authentication("Invalid password"));│
│     }                                                         │
│ } else if room.password.is_some() {                           │
│     return Err(AppError::authentication("Password required"));│
│ }                                                             │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 4: 检查房间状态                                          │
│                                                               │
│ if room.is_expired() {                                        │
│     return Err(AppError::authentication("Room has expired")); │
│ }                                                             │
│ if !room.can_enter() {                                        │
│     return Err(AppError::authentication("Room cannot be entered"));│
│ }                                                             │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 5: 递增访问计数                                          │
│                                                               │
│ let should_increment_view_count = determine_increment();      │
│ if should_increment_view_count {                              │
│     room.current_times_entered += 1;                          │
│     repository.update(&room).await?;                          │
│ }                                                             │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 6: 生成 Access Token                                     │
│                                                               │
│ let (token, claims) = app_state.token_service()               │
│     .issue(&room)?;                                           │
│                                                               │
│ JWT Payload:                                                  │
│ {                                                             │
│   "jti": "uuid-xxx",                                          │
│   "room_id": 1,                                               │
│   "room_name": "myroom-a1b2c3d4",                             │
│   "permission": 7,                                            │
│   "token_type": "access",                                     │
│   "iat": 1705741200,                                          │
│   "exp": 1705748400  // 2小时后                               │
│ }                                                             │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 7: 生成 Refresh Token（可选）                            │
│                                                               │
│ let refresh_token = if payload.with_refresh_token {           │
│     let (rt, rt_claims) = app_state.token_service()           │
│         .issue_refresh_token(&room, &claims.jti)?;            │
│                                                               │
│     // 存储 Refresh Token                                     │
│     let refresh_repo = RefreshTokenRepository::new(...);      │
│     refresh_repo.create(&room, &rt_claims, &claims.jti).await?;│
│                                                               │
│     Some(rt)                                                  │
│ } else {                                                      │
│     None                                                      │
│ };                                                            │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 8: 存储 Token 记录                                       │
│                                                               │
│ let token_repo = RoomTokenRepository::new(...);               │
│ token_repo.create(&room, &claims).await?;                     │
│                                                               │
│ INSERT INTO room_tokens (                                     │
│     room_id, jti, issued_at, expires_at, permission           │
│ ) VALUES (...)                                                │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ Step 9: 返回响应                                              │
│                                                               │
│ Ok(Json(IssueTokenResponse {                                  │
│     access_token: token,                                      │
│     refresh_token,                                            │
│     expires_in: app_state.jwt_config.ttl_seconds,             │
│     token_type: "Bearer".to_string(),                         │
│ }))                                                           │
│                                                               │
│ 响应示例：                                                    │
│ {                                                             │
│   "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...", │
│   "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",│
│   "expires_in": 7200,                                         │
│   "token_type": "Bearer"                                      │
│ }                                                             │
└──────────────────────────────────────────────────────────────┘
```

### 3.3 文件上传流程（分块上传）

```
场景：上传一个 100MB 的大文件，分 10 个分块

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
第一步：准备上传
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

POST /api/v1/rooms/{name}/chunked-uploads/prepare
Authorization: Bearer {access_token}
Body: {
  "files": [{
    "file_name": "large.zip",
    "size": 104857600,           // 100MB
    "total_chunks": 10,
    "chunk_size": 10485760,      // 10MB per chunk
    "file_hash": "sha256-xxx"    // 整个文件的哈希（可选）
  }]
}

┌──────────────────────────────────────────────────────────────┐
│ Handler: prepare_chunked_upload                               │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 1. 验证 Token 和权限                                          │
│    - 解码 JWT                                                 │
│    - 检查黑名单                                               │
│    - 验证 room.permission.can_edit()                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. 检查房间空间                                               │
│    total_size = sum(files.size) = 100MB                       │
│    if !room.can_add_content(total_size) {                     │
│        return Err("Insufficient space")                       │
│    }                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. 创建上传预留记录                                           │
│    upload_token = uuid::new_v4()                              │
│    expires_at = now + upload_reservation_ttl (30分钟)         │
│                                                               │
│    INSERT INTO room_upload_reservations (                     │
│        room_id, token_jti, upload_token,                      │
│        file_manifest, reserved_size, expires_at               │
│    ) VALUES (...)                                             │
│                                                               │
│    同时更新房间的 current_size：                               │
│    UPDATE rooms SET current_size = current_size + 100MB       │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 4. 创建分块上传记录                                           │
│                                                               │
│    INSERT INTO room_chunk_uploads (                           │
│        room_id, upload_token, file_name,                      │
│        total_chunks, chunk_size, total_size,                  │
│        uploaded_chunks, status                                │
│    ) VALUES (                                                 │
│        1, "uuid-xxx", "large.zip",                            │
│        10, 10485760, 104857600,                               │
│        "[]",  // 空数组，表示还没有上传任何分块                │
│        0      // 状态：0=进行中                                │
│    )                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 5. 返回响应                                                   │
│                                                               │
│    {                                                          │
│      "upload_id": "uuid-xxx",                                 │
│      "expires_at": "2024-01-20T10:30:00Z",                    │
│      "files": [{                                              │
│        "upload_token": "uuid-yyy",                            │
│        "file_name": "large.zip",                              │
│        "total_chunks": 10                                     │
│      }]                                                       │
│    }                                                          │
└──────────────────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
第二步：上传分块（重复 10 次，每次上传一个分块）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

POST /api/v1/rooms/{name}/chunked-uploads/{upload_id}/chunks/0
Authorization: Bearer {access_token}
Content-Type: application/octet-stream
X-Chunk-Hash: sha256-chunk-0-hash  // 分块哈希（可选）
Body: [10MB binary data]

┌──────────────────────────────────────────────────────────────┐
│ Handler: upload_chunk                                         │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 1. 验证上传记录                                               │
│    - 查找 room_chunk_uploads                                  │
│    - 检查 upload_token 是否匹配                               │
│    - 检查 chunk_index 是否有效（0-9）                         │
│    - 检查是否已经上传过此分块                                 │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. 读取和验证分块数据                                         │
│    let chunk_data = read_request_body();                      │
│    if chunk_data.len() != expected_chunk_size {               │
│        return Err("Chunk size mismatch")                      │
│    }                                                          │
│                                                               │
│    // 计算分块哈希                                            │
│    let calculated_hash = sha256(chunk_data);                  │
│    if provided_hash != calculated_hash {                      │
│        return Err("Chunk hash mismatch")                      │
│    }                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. 保存分块到临时目录                                         │
│    temp_dir = /tmp/elizabeth/chunks/{upload_id}/              │
│    chunk_file = temp_dir + "chunk_0"                          │
│    write_file(chunk_file, chunk_data)                         │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 4. 更新上传记录                                               │
│    UPDATE room_chunk_uploads SET                              │
│        uploaded_chunks = '[0]',  // 添加分块索引               │
│        updated_at = NOW()                                     │
│    WHERE upload_token = 'uuid-xxx'                            │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 5. 返回响应                                                   │
│    {                                                          │
│      "chunk_index": 0,                                        │
│      "status": "completed",                                   │
│      "uploaded_chunks": [0],                                  │
│      "total_chunks": 10                                       │
│    }                                                          │
└──────────────────────────────────────────────────────────────┘

... 重复上传 chunk_1 到 chunk_9 ...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
第三步：合并文件
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

POST /api/v1/rooms/{name}/chunked-uploads/{upload_id}/merge
Authorization: Bearer {access_token}
Body: {
  "file_hash": "sha256-complete-file-hash"  // 整个文件的哈希
}

┌──────────────────────────────────────────────────────────────┐
│ Handler: merge_chunks                                         │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 1. 验证所有分块都已上传                                       │
│    uploaded_chunks = [0,1,2,3,4,5,6,7,8,9]                    │
│    if uploaded_chunks.len() != total_chunks {                 │
│        return Err("Not all chunks uploaded")                  │
│    }                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. 按顺序合并分块                                             │
│    final_file = storage/rooms/{room_slug}/{uuid}-large.zip    │
│    for i in 0..10 {                                           │
│        chunk_file = temp_dir + f"chunk_{i}"                   │
│        append_file(final_file, read_file(chunk_file))         │
│    }                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. 验证文件哈希                                               │
│    calculated_hash = sha256(final_file)                       │
│    if provided_hash != calculated_hash {                      │
│        delete_file(final_file)                                │
│        return Err("File hash mismatch")                       │
│    }                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 4. 创建 RoomContent 记录                                      │
│    INSERT INTO room_contents (                                │
│        room_id, content_type, display_name,                   │
│        file_path, mime_type, size                             │
│    ) VALUES (                                                 │
│        1, 0, "large.zip",                                     │
│        "{room_slug}/{uuid}-large.zip",                        │
│        "application/zip", 104857600                           │
│    )                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 5. 清理临时文件和更新状态                                      │
│    - 删除临时目录：/tmp/elizabeth/chunks/{upload_id}/          │
│    - 更新上传记录状态为"已完成"                                │
│    - 删除上传预留记录                                         │
│    - 触发 WebSocket 事件：CONTENT_CREATED                     │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 6. 返回响应                                                   │
│    {                                                          │
│      "content_id": 123,                                       │
│      "file_name": "large.zip",                                │
│      "size": 104857600,                                       │
│      "mime_type": "application/zip",                          │
│      "created_at": "2024-01-20T10:25:00Z"                     │
│    }                                                          │
└──────────────────────────────────────────────────────────────┘
```

### 3.4 WebSocket 实时通信流程

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
WebSocket 连接建立
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

客户端：
const ws = new WebSocket('ws://localhost:4092/api/v1/ws');

┌──────────────────────────────────────────────────────────────┐
│ 1. 建立 WebSocket 连接                                        │
│    axum::extract::ws::WebSocketUpgrade                        │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. 升级到 WebSocket 协议                                      │
│    ws.on_upgrade(|socket| handle_socket(socket))              │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. 生成连接 ID                                                │
│    connection_id = uuid::new_v4()                             │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 4. 等待 CONNECT 消息                                          │
│    客户端必须先发送 CONNECT 消息进行认证                       │
└──────────────────────────────────────────────────────────────┘

客户端发送：
{
  "message_type": "CONNECT",
  "payload": {
    "room_name": "myroom-a1b2c3d4",
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }
}

┌──────────────────────────────────────────────────────────────┐
│ 5. 处理 CONNECT 消息                                          │
│    MessageHandler::handle_connect()                           │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 6. 验证 Token                                                 │
│    let claims = token_service.decode(&token)?;                │
│    - 验证签名                                                 │
│    - 检查过期时间                                             │
│    - 检查黑名单                                               │
│    - 验证房间名称匹配                                         │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 7. 注册连接到房间                                             │
│    ConnectionManager::add_connection(                         │
│        room_name, connection_id, sender_channel               │
│    )                                                          │
│                                                               │
│    内部数据结构：                                             │
│    HashMap<RoomName, Vec<(ConnectionId, Sender)>>             │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 8. 发送 CONNECT_ACK                                           │
│    {                                                          │
│      "message_type": "CONNECT_ACK",                           │
│      "payload": {                                             │
│        "success": true,                                       │
│        "message": "Connected successfully"                    │
│      }                                                        │
│    }                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 9. 启动消息循环                                               │
│    - 接收任务：监听客户端消息                                 │
│    - 发送任务：从广播通道接收消息并发送给客户端               │
│    - 心跳任务：定期发送 PING，检测连接活性                    │
└──────────────────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
实时事件推送
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

场景：用户 A 上传了一个文件，通知房间内的其他用户

┌──────────────────────────────────────────────────────────────┐
│ 1. 用户 A 上传文件                                            │
│    POST /api/v1/rooms/{name}/contents                         │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. Handler 处理上传                                           │
│    - 保存文件到存储                                           │
│    - 创建 RoomContent 记录                                    │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. 触发 WebSocket 事件                                        │
│    app_state.connection_manager.broadcast_to_room(             │
│        &room_name,                                            │
│        WsMessage {                                            │
│            message_type: "CONTENT_CREATED",                   │
│            payload: {                                         │
│                "content_id": 123,                             │
│                "file_name": "test.txt",                       │
│                "size": 1024,                                  │
│                "created_by": "user_a"                         │
│            }                                                  │
│        }                                                      │
│    )                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 4. ConnectionManager 广播消息                                 │
│    for (conn_id, sender) in room_connections {                │
│        sender.send(message.clone()).await;                    │
│    }                                                          │
└────────────┬─────────────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────────────┐
│ 5. 各个客户端接收消息                                         │
│    客户端 B、C、D... 都会收到相同的消息                        │
│    - 更新 UI 显示新文件                                       │
│    - 播放通知声音                                             │
│    - 显示提示："用户 A 上传了 test.txt"                       │
└──────────────────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
心跳机制
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

每 30 秒：

客户端 → 服务器：
{
  "message_type": "PING"
}

服务器 → 客户端：
{
  "message_type": "PONG",
  "payload": {
    "timestamp": "2024-01-20T10:30:00Z"
  }
}

如果 60 秒内没有收到任何消息（包括 PONG），客户端认为连接断开，自动重连。
```

---

## 4. 数据模型设计

### 4.1 Entity-Relationship 图

```
┌─────────────────────────┐
│        rooms            │
│─────────────────────────│
│ id (PK)                 │◄─────┐
│ name                    │      │
│ slug (UNIQUE)           │      │ 1:N
│ password (nullable)     │      │
│ status                  │      │
│ max_size                │      │
│ current_size            │      │
│ max_times_entered       │      │
│ current_times_entered   │      │
│ expire_at (nullable)    │      │
│ created_at              │      │
│ updated_at              │      │
│ permission              │      │
└─────────────────────────┘      │
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
         │                       │                       │
┌────────▼────────────┐  ┌───────▼──────────┐  ┌───────▼──────────────┐
│  room_contents      │  │  room_tokens     │  │ room_chunk_uploads   │
│─────────────────────│  │──────────────────│  │──────────────────────│
│ id (PK)             │  │ id (PK)          │  │ id (PK)              │
│ room_id (FK)        │  │ room_id (FK)     │  │ room_id (FK)         │
│ content_type        │  │ jti (UNIQUE)     │  │ upload_token (UK)    │
│ display_name        │  │ issued_at        │  │ file_name            │
│ file_path (null)    │  │ expires_at       │  │ total_chunks         │
│ text_content (null) │  │ permission       │  │ chunk_size           │
│ url (nullable)      │  │ created_at       │  │ total_size           │
│ mime_type (null)    │  └──────────────────┘  │ uploaded_chunks      │
│ size                │                         │ status               │
│ created_at          │                         │ created_at           │
│ updated_at          │                         │ updated_at           │
└─────────────────────┘                         └──────────────────────┘

         ┌───────────────────────┐
         │                       │
┌────────▼─────────────────┐  ┌─▼────────────────────────┐
│ room_refresh_tokens      │  │ room_upload_reservations │
│──────────────────────────│  │──────────────────────────│
│ id (PK)                  │  │ id (PK)                  │
│ room_id (FK)             │  │ room_id (FK)             │
│ jti (UNIQUE)             │  │ token_jti                │
│ access_token_jti         │  │ upload_token (UNIQUE)    │
│ issued_at                │  │ file_manifest            │
│ expires_at               │  │ reserved_size            │
│ last_used_at (nullable)  │  │ expires_at               │
│ revoked (boolean)        │  │ created_at               │
│ created_at               │  └──────────────────────────┘
│ updated_at               │
└──────────────────────────┘
```

### 4.2 数据库索引设计

为了优化查询性能，Elizabeth 在以下字段上创建了索引：

```sql
-- rooms 表
CREATE UNIQUE INDEX idx_rooms_slug ON rooms(slug);
CREATE INDEX idx_rooms_status ON rooms(status);
CREATE INDEX idx_rooms_expire_at ON rooms(expire_at);

-- room_contents 表
CREATE INDEX idx_contents_room_id ON room_contents(room_id);
CREATE INDEX idx_contents_type ON room_contents(content_type);
CREATE INDEX idx_contents_created_at ON room_contents(created_at);

-- room_tokens 表
CREATE UNIQUE INDEX idx_tokens_jti ON room_tokens(jti);
CREATE INDEX idx_tokens_room_id ON room_tokens(room_id);
CREATE INDEX idx_tokens_expires_at ON room_tokens(expires_at);

-- room_refresh_tokens 表
CREATE UNIQUE INDEX idx_refresh_jti ON room_refresh_tokens(jti);
CREATE INDEX idx_refresh_room_id ON room_refresh_tokens(room_id);
CREATE INDEX idx_refresh_access_jti ON room_refresh_tokens(access_token_jti);

-- room_chunk_uploads 表
CREATE UNIQUE INDEX idx_chunk_token ON room_chunk_uploads(upload_token);
CREATE INDEX idx_chunk_room_id ON room_chunk_uploads(room_id);
CREATE INDEX idx_chunk_status ON room_chunk_uploads(status);

-- room_upload_reservations 表
CREATE UNIQUE INDEX idx_reservation_token ON room_upload_reservations(upload_token);
CREATE INDEX idx_reservation_room_id ON room_upload_reservations(room_id);
CREATE INDEX idx_reservation_expires_at ON room_upload_reservations(expires_at);
```

---

## 5. 安全性设计

### 5.1 认证安全

#### JWT Token 设计

- **算法**: HS256（HMAC with SHA-256）
- **密钥长度**: 最小 32 字符，建议 48+ 字符
- **过期时间**:
  - Access Token: 2 小时（可配置）
  - Refresh Token: 7 天（可配置）

#### Token 轮换机制

```rust
// Refresh Token 轮换
// 每次使用 Refresh Token 刷新 Access Token 时，可选地生成新的 Refresh Token
pub async fn refresh_access_token(
    &self,
    refresh_token: &str,
) -> Result<(String, Option<String>)> {
    // 1. 验证 Refresh Token
    let claims = self.token_service.decode(refresh_token)?;

    // 2. 检查是否在黑名单
    if self.blacklist_repo.is_blacklisted(&claims.jti).await? {
        return Err(anyhow!("Refresh token is revoked"));
    }

    // 3. 获取房间信息
    let room = self.room_repo.find_by_id(claims.room_id).await?
        .ok_or_else(|| anyhow!("Room not found"))?;

    // 4. 生成新的 Access Token
    let (new_access_token, new_claims) = self.token_service.issue(&room)?;

    // 5. 如果启用 Token 轮换，生成新的 Refresh Token
    let new_refresh_token = if self.config.enable_refresh_token_rotation {
        // 撤销旧的 Refresh Token
        self.blacklist_repo.add(&claims.jti, claims.exp).await?;

        // 生成新的 Refresh Token
        let (rt, rt_claims) = self.token_service
            .issue_refresh_token(&room, &new_claims.jti)?;
        self.refresh_token_repo.create(&room, &rt_claims, &new_claims.jti).await?;

        Some(rt)
    } else {
        None
    };

    Ok((new_access_token, new_refresh_token))
}
```

### 5.2 密码安全

使用 Argon2 算法进行密码哈希：

```rust
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

// 密码哈希
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(password_hash)
}

// 密码验证
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = PasswordHash::new(hash).ok();
    if let Some(parsed_hash) = parsed_hash {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    } else {
        false
    }
}
```

### 5.3 输入验证

```rust
// 房间名称验证
pub struct RoomNameValidator;

impl RoomNameValidator {
    pub fn validate(name: &str) -> Result<()> {
        // 长度检查
        if name.len() < 1 || name.len() > 64 {
            return Err(AppError::validation("Room name must be 1-64 characters"));
        }

        // 字符检查：允许字母、数字、中文、-_空格
        let re = Regex::new(r"^[\w\p{Han}\s-]+$")?;
        if !re.is_match(name) {
            return Err(AppError::validation(
                "Room name contains invalid characters"
            ));
        }

        Ok(())
    }
}

// 密码验证
pub struct PasswordValidator;

impl PasswordValidator {
    pub fn validate_room_password(password: &str) -> Result<()> {
        // 长度检查
        if password.len() < 6 || password.len() > 128 {
            return Err(AppError::validation("Password must be 6-128 characters"));
        }

        Ok(())
    }
}

// 文件名验证
pub fn sanitize_filename(filename: &str) -> String {
    use sanitize_filename::sanitize;
    sanitize(filename)
}
```

### 5.4 CORS 配置

```rust
use tower_http::cors::{CorsLayer, Any};

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)  // 生产环境应限制具体域名
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .expose_headers([
            header::CONTENT_LENGTH,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-request-id"),
        ])
        .max_age(Duration::from_secs(3600))
}
```

### 5.5 安全响应头

```rust
pub fn security_headers_middleware() -> impl Layer<Route> {
    middleware::from_fn(|req: Request, next: Next| async move {
        let mut response = next.run(req).await;

        // 添加安全头
        response.headers_mut().insert(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        );
        response.headers_mut().insert(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        );
        response.headers_mut().insert(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        );
        response.headers_mut().insert(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );

        response
    })
}
```

---

## 6. 性能优化

### 6.1 数据库连接池

```rust
// 使用 SQLx 连接池
use sqlx::postgres::{PgPool, PgPoolOptions};

pub async fn create_db_pool(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(20)        // 最大连接数
        .min_connections(2)          // 最小连接数
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await
        .context("Failed to connect to database")
}
```

### 6.2 响应压缩

```rust
use tower_http::compression::CompressionLayer;

// 自动压缩响应
let app = Router::new()
    .layer(CompressionLayer::new())
    // ...
```

### 6.3 静态文件缓存

```rust
// 文件下载时设置缓存头
pub async fn download_content(...) -> impl IntoResponse {
    // ...

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),  // 1 小时缓存
    );
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(&format!("\"{}\"", content_id))?,
    );

    (headers, file_stream)
}
```

### 6.4 异步处理

```rust
// 房间清理服务异步运行
pub struct RoomGcService {
    pool: Arc<PgPool>,
    interval: Duration,
}

impl RoomGcService {
    pub fn spawn(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                self.run_gc().await;
                tokio::time::sleep(self.interval).await;
            }
        });
    }

    async fn run_gc(&self) {
        // 清理过期房间
        // 清理过期 Token
        // 清理过期上传预留
    }
}
```

---

## 7. 扩展性设计

### 7.1 存储抽象层

Elizabeth 使用 `opendal` 库提供存储抽象，支持多种存储后端：

```rust
use opendal::{Operator, Scheme};

pub async fn create_storage(config: &StorageConfig) -> Result<Operator> {
    let op = match config.backend {
        StorageBackend::Local => {
            Operator::new(Scheme::Fs)?
                .root(&config.root)
                .finish()
        }
        StorageBackend::S3 => {
            Operator::new(Scheme::S3)?
                .bucket(&config.s3_bucket)
                .region(&config.s3_region)
                .access_key_id(&config.s3_access_key)
                .secret_access_key(&config.s3_secret_key)
                .finish()
        }
        // 支持其他存储后端...
    };

    Ok(op)
}
```

### 7.2 数据库抽象

支持 SQLite 和 PostgreSQL：

```rust
// SQLx 支持多种数据库
#[cfg(feature = "sqlite")]
type DbPool = sqlx::SqlitePool;

#[cfg(feature = "postgres")]
type DbPool = sqlx::PgPool;
```

### 7.3 中间件系统

可扩展的中间件系统：

```rust
pub fn build_app(app_state: AppState) -> Router {
    Router::new()
        .merge(api_routes(app_state.clone()))
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .layer(RequestIdLayer)
        .layer(SecurityHeadersLayer)
        .layer(RateLimitLayer::new(...))
        .layer(TracingLayer::new(...))
        .with_state(app_state)
}
```

---

## 总结

Elizabeth 项目采用了现代化的 Rust
技术栈，通过清晰的分层架构、完善的类型系统、严格的安全措施和高效的性能优化，实现了一个可靠、安全、高性能的文件分享与协作平台。

核心设计特点：

1. **以房间为中心**: 简化权限管理，提升用户体验
2. **无用户系统**: 降低使用门槛，保护隐私
3. **临时性设计**: 自动清理过期数据，减少维护成本
4. **安全优先**: JWT 认证、密码哈希、输入验证、Token 黑名单
5. **高性能**: 异步 I/O、连接池、响应压缩、静态文件缓存
6. **可扩展**: 存储抽象、数据库抽象、中间件系统
