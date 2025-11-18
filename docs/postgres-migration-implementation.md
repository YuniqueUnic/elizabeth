# PostgreSQL Integration & Mixed-Content Remediation Guide

> **Scope**: describe the concrete refactor plan to finish the ongoing
> SQLite→PostgreSQL migration, align repositories with `sqlx::Any`, remove HTTPS
> mixed-content warnings, and restore automated testing.
>
> **Audience**: backend/frontend maintainers who will continue the refactor.

---

## 1. Current Landmarks

1. **DB bootstrap already switched to AnyPool**

   ```rust
   // crates/board/src/db/mod.rs (current trunk)
   pub type DbPool = AnyPool;

   pub async fn run_migrations(pool: &DbPool, url: &str) -> Result<()> {
       let path = match DbKind::detect(url) {
           DbKind::Sqlite => "./migrations",
           DbKind::Postgres => "./migrations_pg",
       };
       sqlx::migrate::Migrator::new(Path::new(path)).await?.run(pool).await
   }
   ```

   _Implication_: application startup must now pass the **database URL** when
   invoking `run_migrations`, otherwise migrations default to SQLite.

2. **Repositories partially ported to `sqlx::query_as`**

   ```rust
   // crates/board/src/repository/room_repository.rs#L1-L120
   sqlx::query_as::<_, Room>("SELECT ... WHERE id = ?")
       .bind(id)
       .fetch_optional(executor)
       .await
   ```

   **Gap**: other repositories still carry `SqlitePool` imports or
   `QueryBuilder<Sqlite>`; the missing `room_token_repository.rs` implementation
   blocks compilation.

3. **Models keep SQLite-only codecs**

   ```rust
   // crates/board/src/models/room/chunk_upload.rs#L33-L72
   impl sqlx::Type<sqlx::Sqlite> for ChunkStatus { .. }
   impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ChunkStatus { .. }
   ```

   _Requirement_: ensure Postgres also understands these enums, either via
   `Type<Postgres>` or string columns.

4. **README already explains dual migrations** (`README.md`
   “数据库选择”section). No changes needed besides linking to this guide.

5. **Tests remain SQLite-specific**

   ```rust
   // crates/board/tests/room_repository_tests.rs#L12-L40
   use sqlx::SqlitePool;
   let pool = SqlitePool::connect(":memory:").await?;
   sqlx::migrate!("./migrations").run(&pool).await?;
   ```

   _Action_: switch these helpers to `AnyPool` or feature-gate Postgres runs via
   env.

---

## 2. Target Architecture

```
┌──────────┐    https     ┌─────────────────┐    http     ┌──────────────┐    TCP    ┌──────────┐
│ Browser  │ ───────────► │ Next.js (proxy) │ ───────────►│ Axum Backend │──────────►│ DB (Pg/Sql)
└──────────┘              │  /api rewrite   │             │  sqlx::Any    │          │ (docker) │
                          └─────────────────┘             └──────────────┘          └──────────┘
```

_Notes_

- Browser must always call `https://box.yunique.top/api/v1`. Mixed content is
  avoided by using relative `NEXT_PUBLIC_API_URL`.
- Axum backend talks to either SQLite or PostgreSQL, chosen via `DATABASE_URL`.

---

## 3. Implementation Steps

### 3.1 Database bootstrap

1. **App startup** (`crates/board/src/lib.rs` or equivalent) must:
   ```rust
   let db_settings = DbPoolSettings::new(cfg.app.database.url.clone());
   let pool = db_settings.create_pool().await?;
   run_migrations(&pool, &cfg.app.database.url).await?;
   ```
2. Ensure `DbPool` is propagated through `AppState` and `Services` as-is (no
   `SqlitePool`).
3. Use environment variable detection (already in `README`) to switch
   migrations.

### 3.2 Repository unification

**Pattern** to apply to every repository file in `crates/board/src/repository`:

```rust
use sqlx::{Any, Executor};

sqlx::query_as::<_, Model>("SQL WHERE id = ?")
    .bind(id)
    .fetch_optional(executor)
    .await;
```

- Replace `QueryBuilder<Sqlite>` with `QueryBuilder<Any>`.
- Replace `last_insert_rowid()` by `RETURNING id`.
- Keep naming consistent (`SqliteRoomRepository` can be renamed `RoomRepository`
  later).

**Status**

| File                                    | Any-ready?    | Actions                                                |
| --------------------------------------- | ------------- | ------------------------------------------------------ |
| `room_repository.rs`                    | ✅            | rename struct later (optional).                        |
| `room_content_repository.rs`            | ✅            | –                                                      |
| `room_token_repository.rs`              | ⚠️ (re-added) | run `cargo fmt` & tests.                               |
| `room_refresh_token_repository.rs`      | ✅            | –                                                      |
| `room_chunk_upload_repository.rs`       | ✅            | –                                                      |
| `room_upload_reservation_repository.rs` | ✅            | ensure `mark_uploaded` returns consistent status enum. |

### 3.3 Model codecs

Files such as `chunk_upload.rs`, `upload_reservation.rs`, `permission.rs`
implement only `Type/Encode/Decode` for SQLite. Options:

- **Option A**: add parallel implementations for `sqlx::Postgres` using the same
  string representation.
- **Option B**: change table schema to store enums as `TEXT` and cast manually
  (no custom codec needed).

Pseudo code (Option A):

```rust
impl sqlx::Type<sqlx::Postgres> for ChunkStatus {
    fn type_info() -> PgTypeInfo { <String as Type<Postgres>>::type_info() }
}
impl<'r> sqlx::Decode<'r, sqlx::Postgres> for ChunkStatus { /* parse string */ }
impl<'q> sqlx::Encode<'q, sqlx::Postgres> for ChunkStatus { /* push string */ }
```

### 3.4 Service & State wiring

- Replace `SqliteRoomRepository` generics in `crates/board/src/services/mod.rs`
  and `AppState` with the new repository struct (optionally rename to
  `RoomRepository`).
- Update tests in `crates/board/src/services` to use
  `DbPoolSettings::new("sqlite::memory:")` + `DbPool` rather than
  `SqlitePool::connect`.

### 3.5 Tests

1. **Default SQLite path** – `cargo test` (no env) should spin up
   `sqlite::memory:` via `DbPoolSettings`.
2. **PostgreSQL matrix** – add `#[cfg_attr(feature = "pg-tests", ignore)]` to
   heavy suites, or provide a helper macro:

```rust
async fn test_pool() -> DbPool {
    let url = std::env::var("DATABASE_URL").unwrap_or("sqlite::memory:".into());
    DbPoolSettings::new(url).create_pool().await.unwrap()
}
```

3. Use docker-compose (already present) to boot Postgres for CI.

### 3.6 Frontend HTTPS validation

1. Rebuild frontend:
   `docker compose build --no-cache frontend && docker compose up -d frontend`.
2. Use Chrome DevTools → Network/Console (or `chrome-devtools` MCP) to ensure:
   - All API requests hit `https://box.yunique.top/api/v1/*`.
   - No `Mixed Content` warnings.

### 3.7 Cleanup

- Remove stale `crates/**/.sqlx/query-*.json` caches (optional but reduces
  noise).
- Regenerate `Cargo.lock` (`cargo metadata` or `cargo check` automatically
  updates once features change).
- Document final steps in CHANGELOG/README.

---

## 4. Execution Checklist (for tracking)

1. [ ] Restore `room_token_repository.rs` (Any-compatible) and run `cargo fmt`.
2. [ ] Search & replace leftovers:
   - `rg -n "SqlitePool" -g '*.rs'`
   - `rg -n 'QueryBuilder::<Sqlite>'`
   - `rg -n 'last_insert_rowid'`
3. [ ] Add Postgres codecs in `chunk_upload.rs`, `upload_reservation.rs`,
       `permission.rs`.
4. [ ] Update state/services/tests to use `DbPool` helpers.
5. [ ] `cargo check` ➜ `cargo test` (SQLite).
6. [ ] `DATABASE_URL=postgres://... cargo test -- --ignored pg` (or a new
       `just test-pg`).
7. [ ] Rebuild frontend & validate HTTPS via DevTools.
8. [ ] Delete `.sqlx/query-*.json` caches.
9. [ ] Commit + document.

---

## 5. Suggested Automation Snippets

- **Python helper to purge `.sqlx` cache**:
  ```python
  # scripts/cleanup_sqlx.py
  import pathlib
  root = pathlib.Path(__file__).resolve().parents[1]
  for json_file in root.rglob(".sqlx/*.json"):
      json_file.unlink()
  ```
- **Dockerized Postgres test spin-up**:
  ```bash
  docker compose up -d postgres
  DATABASE_URL=postgres://postgres:password@localhost:5432/elizabeth \
    cargo test -- --ignored pg
  ```

---

## 6. Appendix: Mixed-Content Verification Flow

```
[Browser HTTPS]
    │ 1. load https://box.yunique.top
    ▼
[Next.js middleware (web/next.config.mjs)]
    │ - rewrites `/api/v1/*` to `INTERNAL_API_URL`
    ▼
[Axum backend]
    │ - handles REST + migrations (AnyPool)
    ▼
[Database]
```

- Use DevTools → Security tab to confirm `Main origin is secure`.
- Failing requests typically show `net::ERR_MIXED_CONTENT`; ensure `.env`
  exports relative URLs to avoid regressions.
