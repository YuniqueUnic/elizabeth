# PostgreSQL 支持添加计划

## 需要的更改

### 1. 依赖更新

在 `crates/board/Cargo.toml` 中添加：

```toml
[dependencies]
# 现有依赖...

# PostgreSQL 支持
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "tls-rustls-aws-lc-rs",
    "postgres",
    "sqlite",
    "migrate",
    "chrono",
] }

# 数据库连接器
[target.'cfg(not(any(target_arch = "aarch64", target_arch = "arm"))'.dependencies]
sqlx-postgres = { version = "0.8", features = ["runtime-tokio", "tls-rustls-aws-lc-rs", "postgres"] }

[target.'cfg(any(target_arch = "aarch64", target_arch = "arm"))'.dependencies]
sqlx-postgres = { version = "0.8", features = ["runtime-tokio", "tls-rustls-aws-lc-rs", "postgres", "sqlite"] }
```

### 2. 配置结构更新

在 `crates/configrs/src/configs/app.rs` 中更新：

```rust
#[derive(Merge, Debug, Clone, SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DatabaseConfig {
    #[default("sqlite:app.db")]
    #[merge(strategy = overwrite_not_empty_string)]
    pub url: String,

    #[default(Some(20))]
    #[merge(strategy = overwrite)]
    pub max_connections: Option<u32>,

    #[default(Some(5))]
    #[merge(strategy = overwrite)]
    pub min_connections: Option<u32>,

    #[default("wal")]
    #[merge(strategy = overwrite_not_empty_string)]
    pub journal_mode: String,

    // 新增：数据库类型配置
    #[default("sqlite")]
    #[merge(strategy = overwrite_not_empty_string)]
    pub database_type: String,

    // 新增：PostgreSQL 特定配置
    #[default(None)]
    #[merge(strategy = overwrite)]
    pub postgres_host: Option<String>,

    #[default(None)]
    #[merge(strategy = overwrite)]
    pub postgres_port: Option<u16>,

    #[default(None)]
    #[merge(strategy = overwrite_not_empty_string)]
    pub postgres_user: Option<String>,

    #[default(None)]
    #[merge(strategy = overwrite_not_empty_string)]
    pub postgres_password: Option<String>,

    #[default(None)]
    #[merge(strategy = overwrite_not_empty_string)]
    pub postgres_database: Option<String>,

    #[default(true)]
    #[merge(strategy = overwrite)]
    pub postgres_ssl: bool,
}
```

### 3. 环境变量支持

添加新的环境变量：

```bash
# SQLite (默认)
DATABASE_URL=sqlite:app.db

# PostgreSQL
DATABASE_URL=postgresql://user:password@localhost:5432/dbname
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_USER=postgres
POSTGRES_PASSWORD=password
POSTGRES_DB=elizabeth
POSTGRES_SSL=true
```

### 4. Docker Compose 更新

```yaml
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: elizabeth
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    networks:
      - elizabeth-network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 10s
      timeout: 5s
      retries: 5

  backend:
    # ... 现有配置
    environment:
      # 数据库配置
      DATABASE_URL: ${DATABASE_URL:-postgresql://postgres:${POSTGRES_PASSWORD:-password}@postgres:5432/${POSTGRES_DB:-elizabeth}}
      POSTGRES_HOST: ${POSTGRES_HOST:-postgres}
      POSTGRES_PORT: ${POSTGRES_PORT:-5432}
      POSTGRES_USER: ${POSTGRES_USER:-postgres}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-password}
      POSTGRES_DB: ${POSTGRES_DB:-elizabeth}
      POSTGRES_SSL: ${POSTGRES_SSL:-true}
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  postgres_data:
```

### 5. 代码更改

在 `crates/board/src/lib.rs` 中更新数据库初始化：

```rust
pub async fn start_server(cfg: &Config) -> anyhow::Result<()> {
    // ... 现有代码

    // 根据数据库类型初始化连接
    let db_settings = match cfg.app.database.database_type.as_str() {
        "postgres" => {
            DbPoolSettings::new(cfg.app.database.url.clone())
                .with_max_connections(cfg.app.database.max_connections)
                .with_min_connections(cfg.app.database.min_connections)
                // PostgreSQL 特定设置
                .with_acquire_timeout(Duration::seconds(30))
                .with_idle_timeout(Duration::seconds(600))
        },
        _ => {
            // SQLite 默认设置
            DbPoolSettings::new(cfg.app.database.url.clone())
                .with_max_connections(cfg.app.database.max_connections)
                .with_min_connections(cfg.app.database.min_connections)
                .with_journal_mode(parse_sqlite_journal_mode(
                    cfg.app.database.journal_mode.as_str(),
                ))
        }
    };

    let db_pool = init_db(&db_settings).await?;
    run_migrations(&db_pool).await?;
    // ... 其余代码保持不变
}
```

## 实施步骤

1. 更新 Cargo.toml 依赖
2. 修改配置结构
3. 更新数据库初始化代码
4. 更新 Docker Compose
5. 添加数据库选择逻辑
6. 测试 SQLite 和 PostgreSQL 连接
7. 更新文档
