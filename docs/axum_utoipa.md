## 主要问题分析

### 1. **Chrono 默认序列化格式**

chrono 的 `DateTime` 默认使用 RFC 3339 格式进行 serde 序列化，`NaiveDateTime`
也使用 ISO 8601 格式。你的 diff 中使用 `DisplayFromStr` 会导致序列化为 `Display`
格式 (`YYYY-MM-DD HH:MM:SS.ffffff`),这**不是标准的 ISO 8601 格式**(缺少 `T`
分隔符),可能导致客户端解析问题。

### 2. **utoipa chrono 特性的自动支持**

当启用 utoipa 的 `chrono` 特性后，`DateTime` 和 `NaiveDateTime` 会自动生成
`format: date-time`(遵循 RFC3339/ISO-8601)。你的代码中**不需要**手动添加
`#[schema(value_type = String, format = DateTime)]`,这反而会覆盖自动生成的规范格式。

### 3. **SQLite DATETIME 映射**

SQLite 的 DATETIME 可以用三种类型表示 (TEXT、REAL、INTEGER),sqlx
会根据存储格式自动推断。NaiveDateTime 可以处理 SQLite
格式的字符串，但建议统一使用 RFC 3339 格式存储以保持一致性。

## 优化后的指南

````markdown
# Axum 时间字段序列化最佳实践指南 (2025-10-16)

## 重构建议

### ❌ **不推荐**:使用 `DisplayFromStr`

```rust
#[serde_as]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct Room {
    #[serde_as(as = "DisplayFromStr")]  // 输出："2025-10-16 12:30:45.123"
    #[schema(value_type = String, format = DateTime)]
    pub created_at: NaiveDateTime,
}
```
````

**问题**:序列化为非标准格式，缺少 `T` 分隔符，可能被客户端拒绝。

### ✅ **推荐**:使用 chrono 默认序列化

```rust
#[derive(Serialize, Deserialize, ToSchema, FromRow)]
pub struct Room {
    // 无需额外注解！
    pub created_at: NaiveDateTime,  // 自动输出："2025-10-16T12:30:45.123"
}
```

**配置要求**:

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
utoipa = { version = "5.4", features = ["chrono"] }
sqlx = { version = "0.8", features = ["chrono", "sqlite"] }
```

## 关键配置说明

### 1. **utoipa chrono 特性**

- **自动映射**:`NaiveDateTime` → OpenAPI `string` + `format: date-time`
- **无需手动**:`#[schema(value_type = ...)]` 会覆盖自动推断

### 2. **chrono 默认序列化**

| 类型            | 格式     | 示例                          |
| --------------- | -------- | ----------------------------- |
| `DateTime<Utc>` | RFC 3339 | `2025-10-16T12:30:45.123456Z` |
| `NaiveDateTime` | ISO 8601 | `2025-10-16T12:30:45.123456`  |
| `NaiveDate`     | ISO 8601 | `2025-10-16`                  |

### 3. **SQLite 存储建议**

```sql
CREATE TABLE rooms (
    created_at TEXT NOT NULL  -- 存储为 ISO 8601 字符串
);
```

**Rust 侧**:

```rust
// 插入
let now = Utc::now().naive_utc();  // 始终使用 UTC
sqlx::query!("INSERT INTO rooms (created_at) VALUES (?)", now)
    .execute(&pool).await?;

// 查询 (自动映射)
#[derive(FromRow)]
struct Room {
    created_at: NaiveDateTime,  // sqlx 自动解析
}
```

## 特殊需求处理

### 如需自定义格式 (如 SQLite 传统格式)

```rust
use serde::{Deserialize, Deserializer, Serializer};

mod sqlite_format {
    use super::*;
    use chrono::NaiveDateTime;

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(dt: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        s.serialize_str(&dt.format(FORMAT).to_string())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<NaiveDateTime, D::Error>
    where D: Deserializer<'de> {
        let s: String = Deserialize::deserialize(d)?;
        NaiveDateTime::parse_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Room {
    #[serde(with = "sqlite_format")]
    pub created_at: NaiveDateTime,
}
```

## 推荐实施步骤

1. **移除所有 `serde_as` 和 `DisplayFromStr`**,依赖 chrono 默认序列化
2. **删除手动 `#[schema(...)]` 注解**,让 utoipa 自动推断
3. **统一使用 UTC**:入库前转换为 `NaiveDateTime`,业务层再转换时区
4. **数据库迁移**:将现有 `YYYY-MM-DD HH:MM:SS` 格式转换为 ISO 8601

## 参考

- [Chrono Serde 文档](https://docs.rs/chrono/latest/chrono/serde/)
- [utoipa chrono 特性](https://docs.rs/utoipa/latest/utoipa/#chrono)
- [SQLx 类型映射](https://docs.rs/sqlx/latest/sqlx/sqlite/types/)

````
## 你的 diff 需要修改的地方

```diff
-#[serde_as]
 #[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
 pub struct Room {
     // ... 其他字段 ...
-    #[serde_as(as = "Option<DisplayFromStr>")]
-    #[schema(value_type = Option<String>, format = DateTime)]
     pub expire_at: Option<NaiveDateTime>,
-    #[serde_as(as = "DisplayFromStr")]
-    #[schema(value_type = String, format = DateTime)]
     pub created_at: NaiveDateTime,
-    #[serde_as(as = "DisplayFromStr")]
-    #[schema(value_type = String, format = DateTime)]
     pub updated_at: NaiveDateTime,
     // ... 其他字段 ...
 }
````

**只需保留 `derive` 宏即可，让 chrono 和 utoipa 自动处理!**
这样既简洁又符合标准。
