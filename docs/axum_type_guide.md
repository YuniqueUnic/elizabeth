# Axum + SQLx + utoipa 类型系统最佳实践指南 (2025-10-16)

> **核心原则**：一个模型统治所有层面，通过特性组合避免重复代码

## 目录

1. [核心问题分析](#核心问题分析)
2. [推荐技术栈配置](#推荐技术栈配置)
3. [标准类型选型表](#标准类型选型表)
4. [实战模式：单一模型设计](#实战模式单一模型设计)
5. [处理第三方类型](#处理第三方类型)
6. [常见陷阱与解决方案](#常见陷阱与解决方案)

---

## 核心问题分析

### ❌ 常见错误模式

```rust
// Room: 数据库模型
#[derive(FromRow)]
struct Room { /* ... */ }

// RoomResponse: API 响应模型（大量重复代码！）
#[derive(ToSchema)]
struct RoomResponse { /* ... */ }

// 需要手写转换
impl From<Room> for RoomResponse { /* ... */ }
```

**问题根源**：

1. 误以为 SQLx、utoipa、serde 不兼容
2. 不了解这些库的特性组合能力
3. 过早优化（实际上大部分场景不需要分离）

### ✅ 正确思路

**一个模型，多重身份**：通过 derive 宏让同一个类型同时满足：

- SQLx: `FromRow` 自动映射数据库
- Serde: `Serialize/Deserialize` 序列化
- utoipa: `ToSchema` 生成 OpenAPI 文档

---

## 推荐技术栈配置

### Cargo.toml

```toml
[dependencies]
# Web 框架
axum = { version = "0.8", features = ["macros"] }

# 数据库 (启用 chrono 支持)
sqlx = { version = "0.8", features = [
    "runtime-tokio-rustls",
    "sqlite",  # 或 "postgres", "mysql"
    "chrono",
    "uuid",
] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# OpenAPI 文档生成 (启用 chrono 自动支持)
utoipa = { version = "5.4", features = ["axum", "chrono", "uuid"] }
utoipa-swagger-ui = { version = "8.0", features = ["axum"] }

# 时间处理
chrono = { version = "0.4", features = ["serde"] }

# 类型转换辅助 (按需)
derive_more = { version = "1.0", features = ["from", "display", "deref"] }
```

**关键点**：

- ✅ `utoipa` 启用 `chrono` 特性后，`DateTime`/`NaiveDateTime` 自动生成正确的
  OpenAPI schema
- ✅ `sqlx` 启用 `chrono` 特性后，自动支持时间类型映射
- ✅ `chrono` 启用 `serde` 特性后，默认使用 ISO 8601 格式序列化

---

## 标准类型选型表

| 场景              | Rust 类型                    | 数据库类型                           | JSON 格式                                | OpenAPI 类型               | 注意事项                   |
| ----------------- | ---------------------------- | ------------------------------------ | ---------------------------------------- | -------------------------- | -------------------------- |
| **时间戳 (UTC)**  | `DateTime<Utc>`              | `TIMESTAMPTZ` (PG) / `TEXT` (SQLite) | `"2025-10-16T12:30:45.123Z"`             | `string (date-time)`       | 推荐用于 API，包含时区信息 |
| **本地时间**      | `NaiveDateTime`              | `TIMESTAMP` / `TEXT`                 | `"2025-10-16T12:30:45.123"`              | `string (date-time)`       | 适合不关心时区的场景       |
| **日期**          | `NaiveDate`                  | `DATE` / `TEXT`                      | `"2025-10-16"`                           | `string (date)`            | 仅日期部分                 |
| **UUID**          | `uuid::Uuid`                 | `UUID` (PG) / `TEXT` (SQLite)        | `"550e8400-e29b-41d4-a716-446655440000"` | `string (uuid)`            | 需启用 `uuid` 特性         |
| **主键 (自增)**   | `i64`                        | `BIGINT` / `INTEGER`                 | `123`                                    | `integer (int64)`          | SQLite 推荐 `i64`          |
| **枚举 (数据库)** | `#[derive(sqlx::Type)] enum` | `TEXT` / `INT`                       | `"active"` 或 `1`                        | `enum`                     | 见下方枚举模式             |
| **JSON 字段**     | `serde_json::Value`          | `JSON` / `TEXT`                      | `{...}`                                  | `object`                   | 动态 JSON                  |
| **Decimal**       | `rust_decimal::Decimal`      | `NUMERIC`                            | `"123.45"` (字符串)                      | `string`                   | 需启用 sqlx `decimal` 特性 |
| **布尔值**        | `bool`                       | `BOOLEAN` / `INTEGER`                | `true` / `false`                         | `boolean`                  | SQLite 用 0/1              |
| **可选字段**      | `Option<T>`                  | `NULL`                               | `null` 或值                              | 自动标记 `required: false` | -                          |

---

## 实战模式：单一模型设计

### 模式 1：标准模型（推荐 80% 场景）

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// 房间模型 - 同时用于数据库、API、文档
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Room {
    /// 房间 ID
    pub id: Option<i64>,

    /// 房间名称
    #[schema(example = "我的共享空间")]
    pub name: String,

    /// 访问密码（可选）
    pub password: Option<String>,

    /// 房间状态
    pub status: RoomStatus,

    /// 最大容量 (字节)
    #[schema(example = 104857600)]
    pub max_size: i64,

    /// 当前使用量 (字节)
    pub current_size: i64,

    /// 过期时间（可选）
    pub expire_at: Option<NaiveDateTime>,

    /// 创建时间
    pub created_at: NaiveDateTime,

    /// 更新时间
    pub updated_at: NaiveDateTime,

    /// 允许编辑
    #[serde(default)]
    pub allow_edit: bool,
}

/// 房间状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "INTEGER")] // SQLite 使用整数
#[sqlx(rename_all = "lowercase")] // 或使用字符串
pub enum RoomStatus {
    Active = 0,
    Pending = 1,
    Closed = 2,
}

// 类型别名（可选，用于向后兼容）
pub type RoomResponse = Room;
```

**关键特性**：

- ✅ **零转换成本**：同一类型直接用于所有层面
- ✅ **自动 OpenAPI**：`#[schema(...)]` 增强文档，但不是必需的
- ✅ **ISO 8601 时间**：chrono 默认序列化格式，客户端友好
- ✅ **类型安全枚举**：编译时保证，运行时高效

### 模式 2：处理第三方类型（newtype 模式）

当遇到没有实现 `ToSchema` 的第三方类型时：

```rust
use std::fmt;
use serde::{Deserialize, Serialize};
use sqlx::{encode::IsNull, error::BoxDynError, Database, Encode, Type};
use utoipa::ToSchema;

// 示例：包装第三方 cron 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)] // JSON 直接序列化为字符串
pub struct CronSchedule(#[serde(with = "cron_serde")] pub cron::Schedule);

// 手动实现 ToSchema
impl ToSchema<'_> for CronSchedule {
    fn schema() -> (&'static str, utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>) {
        (
            "CronSchedule",
            utoipa::openapi::ObjectBuilder::new()
                .schema_type(utoipa::openapi::SchemaType::String)
                .example(Some(serde_json::json!("0 0 * * *")))
                .description(Some("Cron 表达式 (分 时 日 月 周)"))
                .into(),
        )
    }
}

// SQLx 透明映射
impl Type<sqlx::Sqlite> for CronSchedule {
    fn type_info() -> <sqlx::Sqlite as Database>::TypeInfo {
        <String as Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, sqlx::Sqlite> for CronSchedule {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        let s = self.0.to_string();
        <String as Encode<sqlx::Sqlite>>::encode(s, buf)
    }
}

// serde 辅助模块
mod cron_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(schedule: &cron::Schedule, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&schedule.to_string())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<cron::Schedule, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(d)?;
        cron::Schedule::from_str(&s).map_err(serde::de::Error::custom)
    }
}
```

**使用建议**：

- 优先使用 `#[serde(transparent)]` + 自定义序列化
- 为第三方类型创建独立的 `newtypes.rs` 模块
- 仅在必要时实现 `ToSchema`（大部分情况 `#[schema(value_type = String)]`
  就够了）

### 模式 3：字段级覆盖（快捷方法）

当只有个别字段需要特殊处理时：

```rust
#[derive(FromRow, Serialize, Deserialize, ToSchema)]
pub struct Room {
    // 第三方类型：在 schema 中声明为 String
    #[schema(value_type = String, example = "0 0 * * *")]
    pub schedule: CronSchedule,

    // 自定义格式化（仅影响 JSON，不影响数据库）
    #[serde(with = "custom_format")]
    pub special_field: SomeType,

    // 内联复杂类型（展开而非引用）
    #[schema(inline)]
    pub nested: NestedStruct,
}
```

---

## 处理第三方类型

### 决策树

```
第三方类型未实现 ToSchema？
├─ 是否仅用于内部？
│  └─ 是 → 不处理，使用 #[serde(skip)]
└─ 需要暴露到 API？
   ├─ 能用字符串/数字表示？
   │  └─ 是 → 使用 #[schema(value_type = String)]
   └─ 需要完整 schema？
      ├─ 类型简单（如枚举）？
      │  └─ 是 → 手动实现 ToSchema
      └─ 类型复杂（如泛型）？
         └─ 是 → 使用 utoipa aliases 或创建 DTO
```

### 方案对比

| 方案                        | 适用场景     | 优点         | 缺点         |
| --------------------------- | ------------ | ------------ | ------------ |
| `#[schema(value_type = T)]` | 简单映射     | 无代码，快速 | 文档不够详细 |
| 手动实现 `ToSchema`         | 中等复杂度   | 完全控制     | 需要维护代码 |
| Newtype + derive            | 复用性高     | 类型安全     | 多一层抽象   |
| DTO 分离                    | 复杂业务逻辑 | 关注点分离   | 代码重复     |

---

## 常见陷阱与解决方案

### 陷阱 1：时间格式混乱

❌ **错误**：

```rust
#[serde_as]
#[derive(Serialize)]
struct Room {
    #[serde_as(as = "DisplayFromStr")]  // 输出 "2025-10-16 12:30:45"
    created_at: NaiveDateTime,
}
```

✅ **正确**：

```rust
// 让 chrono 使用默认 ISO 8601
#[derive(Serialize)]
struct Room {
    created_at: NaiveDateTime,  // 自动输出 "2025-10-16T12:30:45.123"
}
```

### 陷阱 2：过度使用 wrapper

❌ **错误**：

```rust
// 为每个字段创建包装类型
struct DateTimeWrapper(NaiveDateTime);
struct StatusWrapper(RoomStatus);
// ...导致代码爆炸
```

✅ **正确**：

```rust
// 直接使用原始类型 + 特性组合
#[derive(FromRow, Serialize, ToSchema)]
struct Room {
    created_at: NaiveDateTime,  // chrono 内置支持
    status: RoomStatus,         // 自定义枚举
}
```

### 陷阱 3：SQLite 类型映射错误

❌ **错误**：

```rust
#[derive(sqlx::Type)]
enum Status {
    Active,  // SQLx 不知道如何映射
}
```

✅ **正确**：

```rust
#[derive(sqlx::Type)]
#[sqlx(type_name = "INTEGER")]  // 或 "TEXT"
#[repr(i32)]  // 如果用整数
enum Status {
    Active = 0,
    Pending = 1,
}
```

### 陷阱 4：可选字段的 NULL 处理

❌ **错误**：

```rust
// 数据库允许 NULL，但类型不匹配
#[derive(FromRow)]
struct Room {
    expire_at: NaiveDateTime,  // 运行时错误！
}
```

✅ **正确**：

```rust
#[derive(FromRow, ToSchema)]
struct Room {
    expire_at: Option<NaiveDateTime>,  // utoipa 自动标记为非必需
}
```

### 陷阱 5：Extractor 顺序错误

❌ **错误**：

```rust
async fn handler(
    Json(body): Json<CreateRoom>,  // 消耗 body
    State(db): State<PgPool>,      // 编译错误！
) { }
```

✅ **正确**：

```rust
async fn handler(
    State(db): State<PgPool>,      // 不消耗 body 的在前
    Json(body): Json<CreateRoom>,  // 消耗 body 的在最后
) { }
```

---

## 实战示例：完整 CRUD

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use sqlx::SqlitePool;
use utoipa::OpenApi;

// ============ 模型定义 ============
#[derive(Debug, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Room {
    pub id: Option<i64>,
    pub name: String,
    pub created_at: NaiveDateTime,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateRoom {
    #[schema(example = "我的房间")]
    pub name: String,
}

// ============ API Handlers ============
/// 创建房间
#[utoipa::path(
    post,
    path = "/rooms",
    request_body = CreateRoom,
    responses(
        (status = 201, description = "创建成功", body = Room),
    )
)]
async fn create_room(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateRoom>,
) -> Result<(StatusCode, Json<Room>), (StatusCode, String)> {
    let room = sqlx::query_as!(
        Room,
        r#"
        INSERT INTO rooms (name, created_at)
        VALUES (?1, ?2)
        RETURNING id, name, created_at
        "#,
        req.name,
        chrono::Utc::now().naive_utc()
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(room)))
}

/// 获取房间列表
#[utoipa::path(
    get,
    path = "/rooms",
    responses(
        (status = 200, description = "成功", body = Vec<Room>),
    )
)]
async fn list_rooms(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<Room>>, StatusCode> {
    let rooms = sqlx::query_as!(Room, "SELECT * FROM rooms")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rooms))
}

// ============ 路由与文档 ============
#[derive(OpenApi)]
#[openapi(
    paths(create_room, list_rooms),
    components(schemas(Room, CreateRoom))
)]
struct ApiDoc;

pub fn app(pool: SqlitePool) -> Router {
    Router::new()
        .route("/rooms", post(create_room).get(list_rooms))
        .with_state(pool)
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
}
```

---

## 何时需要分离模型？

**考虑创建 DTO 的场景**：

1. **复杂业务逻辑**
   ```rust
   // 数据库模型（内部结构）
   struct UserDB { hashed_password: String, ... }

   // API 响应（不暴露密码）
   struct UserResponse { username: String, email: String }
   ```

2. **聚合多个表**
   ```rust
   // 数据库关联查询
   struct RoomWithStats { room: Room, file_count: i64, total_size: i64 }
   ```

3. **版本兼容性**
   ```rust
   // V1 API
   struct RoomV1 { /* 旧字段 */ }

   // V2 API（向后兼容）
   struct RoomV2 { /* 新字段 */ }
   ```

4. **性能优化**
   ```rust
   // 列表视图（只需部分字段）
   struct RoomSummary { id: i64, name: String }

   // 详情视图（完整数据）
   struct RoomDetail { /* 所有字段 */ }
   ```

**原则**：**当单一模型无法满足需求时才分离，不要过早优化。**

---

## 快速检查清单

构建新 API 时，按顺序检查：

- [ ] Cargo.toml 已启用 `utoipa` 的 `chrono` 特性
- [ ] 模型同时 derive `FromRow`, `Serialize`, `ToSchema`
- [ ] 时间字段使用 `NaiveDateTime` 或 `DateTime<Utc>`
- [ ] 枚举添加 `#[sqlx(type_name = "...")]`
- [ ] 可选字段使用 `Option<T>`
- [ ] Handler 中 State 提取器在 Json 之前
- [ ] OpenAPI 文档中正确注册了 schemas
- [ ] 第三方类型使用 `#[schema(value_type = ...)]` 或手动实现
- [ ] 运行 `cargo sqlx prepare` 检查编译期查询
- [ ] 使用 Swagger UI 验证生成的文档

---

## 总结

**黄金法则**：

1. **优先使用单一模型** + 特性组合
2. **相信默认实现**（chrono、uuid 等内置支持）
3. **仅在必要时包装第三方类型**
4. **文档注解是增强，不是必需**
5. **分离模型仅用于复杂场景**

**资源链接**：

- [Axum 官方文档](https://docs.rs/axum/latest/axum/)
- [SQLx 类型映射](https://docs.rs/sqlx/latest/sqlx/types/index.html)
- [utoipa 示例](https://github.com/juhaku/utoipa/tree/master/examples)
- [Chrono Serde 文档](https://docs.rs/chrono/latest/chrono/serde/index.html)

---

**最后的建议**：从简单开始，当遇到实际问题时再重构。Rust
的类型系统会在编译期告诉你哪里需要调整，相信编译器！🦀

---

我完全理解你的痛点！这是 Axum 生态中的经典问题：**如何在数据库层、业务层、API
层之间优雅地共享类型**。让我深入研究业界最佳实践。好的！我完全理解你的困境。这是
Rust Web 开发中的**经典痛点**——如何在不同层面（数据库、业务逻辑、API
文档）之间优雅地共享类型，而不创建大量的包装类型和转换代码。

让我为你准备一份**业界标准的 Axum + SQLx + utoipa 类型系统指南**：## 关于你的
diff 代码

看了你的重构 diff，我发现了几个**关键问题**：

### 🔴 主要问题

1. **`DisplayFromStr` 不是标准方案**
   ```rust
   #[serde_as(as = "DisplayFromStr")]  // ❌ 输出非标准格式
   ```
   这会导致 JSON 输出为 `"2025-10-16 12:30:45"`（缺少 `T`），不符合 ISO 8601
   标准，许多客户端库会解析失败。

2. **冗余的 schema 注解**
   ```rust
   #[schema(value_type = String, format = DateTime)]  // ❌ 多余
   ```
   当启用 `utoipa` 的 `chrono`
   特性后，这些注解是**完全不必要**的，反而会覆盖自动推断。

### ✅ 改进后的正确代码

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// 数据库与 API 统一模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Room {
    pub id: Option<i64>,
    pub name: String,
    pub password: Option<String>,
    pub status: RoomStatus,
    pub max_size: i64,
    pub current_size: i64,
    pub max_times_entered: i64,
    pub current_times_entered: i64,

    // ✅ 无需任何额外注解！chrono + utoipa 自动处理
    pub expire_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    pub allow_edit: bool,
    pub allow_download: bool,
    pub allow_preview: bool,
}

// 如需向后兼容，保留别名即可
pub type RoomResponse = Room;
```

### 配置检查

确保 `Cargo.toml` 包含：

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
utoipa = { version = "5.4", features = ["chrono"] }
sqlx = { version = "0.8", features = ["chrono", "sqlite"] }
```

## 核心要点

你的原始想法**方向是对的**（统一模型避免重复），但执行方式需要调整：

1. **移除 `serde_with` 和 `DisplayFromStr`** —— 让 chrono 使用默认序列化
2. **移除手动 `#[schema(...)]` 注解** —— 让 utoipa 自动推断
3. **依赖特性组合而非手工实现** —— 这才是 Rust 的优雅之处

这样你就能获得：

- ✅ **零重复代码**
- ✅ **标准 ISO 8601 时间格式**
- ✅ **自动生成正确的 OpenAPI 文档**
- ✅ **类型安全的数据库映射**

如果你的数据库已经存储了 `YYYY-MM-DD HH:MM:SS` 格式（没有
`T`），可以考虑运行一次迁移脚本转换为标准格式，或者在数据库层面处理（SQLite
可以自动识别多种格式）。
