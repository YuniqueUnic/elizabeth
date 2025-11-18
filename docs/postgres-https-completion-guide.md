# PostgreSQL & HTTPS Completion Implementation Guide (2025-11-18)

## Scope

- Consolidate the ongoing AnyPool migration so both SQLite and PostgreSQL share
  the same repositories, services, and migrations.
- Close the HTTPS mixed-content incident by enforcing relative API URLs across
  the web bundle and verifying via DevTools.
- Provide executable steps for automated test coverage (SQLite + Postgres) and
  repository cleanup (.sqlx caches, Cargo.lock).

## 1. Snapshot & Known Gaps

| Area                   | Code Reference                                                                                                     | Gap / Observation                                                                                                                                                                                                            |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Server bootstrap       | `crates/board/src/lib.rs:63-103`                                                                                   | `run_migrations(&db_pool)` is called without the URL parameter required by `db::run_migrations`, and `.with_journal_mode(...)` is invoked even though `DbPoolSettings` has no such method.                                   |
| DbPool config          | `crates/board/src/db/mod.rs:11-118`                                                                                | `run_migrations` already switches between `./migrations` and `./migrations_pg`, but callers must pass the _same_ URL used for pool creation; add a helper to stash the URL inside `DbPoolSettings`.                          |
| Services wiring        | `crates/board/src/services/mod.rs:9-74`                                                                            | Struct fields and constructors still use `Sqlite*` names even though the implementations rely on `DbPool=Any` internally; rename them to database-agnostic aliases before exporting through `AppState`.                      |
| Application state      | `crates/board/src/state.rs:19-78`                                                                                  | `AppState::new` receives `Arc<DbPool>` but service tests build pools via `init_db` + SQLite-only settings; adopt the same DbPool helper used in production.                                                                  |
| Repositories           | `crates/board/src/repository/*.rs`                                                                                 | Most files already use `sqlx::query_as::<_, Model>()` with `?` binds and `RETURNING id`; confirm `room_token_repository.rs` (lines 1-118) matches the pattern and rename structs away from `Sqlite*` to reflect Any support. |
| Model codecs           | `crates/board/src/models/room/chunk_upload.rs:1-120`, `room/upload_reservation.rs:1-80`, `room/permission.rs:1-80` | Only SQLite `Type/Encode/Decode` implementations exist; PostgreSQL either needs matching trait impls or the schema must store enums as strings/ints to avoid custom codecs.                                                  |
| Tests                  | `crates/board/tests/room_repository_tests.rs:8-250`, `crates/board/src/services/auth_service.rs:216-520`           | Helpers spawn `SqlitePool::connect(":memory:")` directly; replace with `DbPoolSettings::new("sqlite::memory:")` and expose a Pg-aware test harness guarded by env vars.                                                      |
| Frontend HTTPS         | `web/lib/config.ts:7-78`, `web/next.config.mjs:1-45`, `Dockerfile.frontend:35-52`                                  | Runtime already prefers `NEXT_PUBLIC_API_URL=/api/v1` and rewrites via `INTERNAL_API_URL`, but production verification (DevTools Network/Security) is still pending after rebuilding the image.                              |
| Mixed-content playbook | `docs/https-proxy-unification.md`                                                                                  | Document prescribes relative browser URLs and internal proxies; follow it verbatim when auditing `.env` and Docker compose overrides.                                                                                        |
| Reference guide        | `docs/postgres-migration-implementation.md`                                                                        | Explains the design target; this implementation guide decomposes it into concrete code edits plus validation scripts.                                                                                                        |
| Cache files            | `crates/board/.sqlx/query-*.json`                                                                                  | Generated stubs describe the previous Sqlite schema; delete them after sqlx check passes on both DBs to avoid drift.                                                                                                         |

## 2. Database bootstrap & migration orchestration

### 2.1 `start_server` needs Db URL aware migrations

- `start_server` currently composes a pool via
  `DbPoolSettings::new(...).with_journal_mode(...)` and then calls
  `run_migrations(&db_pool)` (see `crates/board/src/lib.rs:63-103`). Replace it
  with a URL-centric flow that works for both Sqlite and Postgres.
- Introduce either `DbPoolSettings::with_sqlite_journal_mode(...)` _or_ move the
  journal choice into `DbPoolSettings::new` so `.with_journal_mode` is no longer
  needed; this keeps the builder API valid.
- Always pass the same URL when invoking migrations:

```rust
let db_url = cfg.app.database.url.clone();
let pool_settings = DbPoolSettings::new(&db_url)
    .with_max_connections(cfg.app.database.max_connections)
    .with_min_connections(cfg.app.database.min_connections);
let pool = pool_settings.create_pool().await?;
run_migrations(&pool, &db_url).await?;
let db_pool = Arc::new(pool);
```

- Forward `Arc<DbPool>` plus `db_url` to `AppState::new` so later subsystems
  (e.g., background jobs) can re-run migrations when they boot independently.

### 2.2 Migration selection flow

```
┌──────────────────┐   create_pool   ┌──────────────────────┐   run_migrations    ┌────────────────────┐
│ AppConfig.database│ ─────────────► │ DbPoolSettings / Any │ ───────────────────► │ ./migrations vs pg │
└──────────────────┘                 │  (sqlite or postgres)│                      │ (detected by URL)   │
                                     └──────────────────────┘                      └────────────────────┘
                                              │                                             │
                                              ▼                                             ▼
                                     ┌──────────────────────┐                     ┌────────────────────┐
                                     │ Arc<DbPool>          │  injected into     │ AppState + Services│
                                     └──────────────────────┘  handlers/tasks    └────────────────────┘
```

- The flow diagram mirrors `crates/board/src/db/mod.rs:11-118`. Ensure every
  consumer (CLI commands, tests, background workers) calls
  `run_migrations(&pool, url)` exactly once after establishing the connection
  pool.
- Update `README.md` "数据库选择" to reference this flow and to highlight that
  Postgres uses `./migrations_pg` while SQLite stays on `./migrations`. Link
  back to this file and to `docs/postgres-migration-implementation.md`.

## 3. Repository and model parity

### 3.1 Repository checklist

| Repository                                                  | File                                                         | Status                                                         | Next Action                                                                                                                                |
| ----------------------------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Rooms                                                       | `crates/board/src/repository/room_repository.rs`             | Uses `sqlx::query_as::<_, Room>()` and `RETURNING id` already. | Rename `SqliteRoomRepository` → `RoomRepository`, expose via `pub use` so services/tests stop depending on a Sqlite-specific type.         |
| Room tokens                                                 | `crates/board/src/repository/room_token_repository.rs:1-118` | Recently restored as Any-compatible.                           | Keep the `fetch_optional` helpers private but rename struct/trait exports; add Pg-specific regression tests once integration layer exists. |
| Content, refresh tokens, chunk uploads, upload reservations | `crates/board/src/repository/room_*`                         | Some files still alias structs as `Sqlite*`.                   | Update type aliases and ensure `QueryBuilder::<Any>` is used everywhere (already true for `room_content_repository.rs:184-210`).           |

Standardized insert/update pattern (applies to every repo):

```rust
pub async fn create(&self, model: &Model) -> Result<Model> {
    let mut tx = self.pool.begin().await?;
    let id: i64 = sqlx::query_scalar(
        r#"INSERT ... VALUES (?) RETURNING id"#,
    )
    .bind(...)
    .fetch_one(&mut *tx)
    .await?;
    let created = Self::fetch_by_id_or_err(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(created)
}
```

### 3.2 Model codec bridge

- `ChunkStatus` (`crates/board/src/models/room/chunk_upload.rs:1-80`),
  `UploadStatus` (`room/upload_reservation.rs:1-70`), and `RoomPermission`
  (`room/permission.rs:1-120`) implement only SQLite traits. Add matching
  `Type<Postgres>`, `Encode<Postgres>`, and `Decode<Postgres>` using the
  existing string/bitflag representations.
- Pseudocode for enum codecs:

```rust
impl sqlx::Type<sqlx::Postgres> for ChunkStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <&str as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}
impl<'r> sqlx::Decode<'r, sqlx::Postgres> for ChunkStatus {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let raw = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        raw.parse().map_err(|e: String| e.into())
    }
}
```

- Alternatively, adjust migrations so these columns are stored as TEXT/INTEGER
  for both DBs and drop custom codecs entirely. Either path must be reflected in
  both `migrations/` and `migrations_pg/`.

### 3.3 QueryBuilder usage

- For bulk deletes (e.g., `room_content_repository.rs:184-210`), stick with
  `QueryBuilder::<Any>`; SQL macros such as `builder.push_bind` already map to
  the right positional syntax for SQLite (`?`) and Postgres (`$1`). No
  additional per-database code is necessary once the builder type is unified.

## 4. Services, state, and dependency injection

- `crates/board/src/services/mod.rs:9-74` exposes
  `room_repository: Arc<SqliteRoomRepository>` plus refresh/token repos with
  Sqlite-prefixed names. Rename the structs (or export type aliases) so
  `Services` only deals with `RoomRepository`, `RoomRefreshTokenRepository`, and
  `TokenBlacklistRepository` backed by AnyPool.
- Update `AppState::new` (`crates/board/src/state.rs:19-74`) to accept
  `Arc<DbPool>` plus an optional reference to the database URL; this allows
  background tasks to re-use migrations and fosters consistency between
  production and tests.
- Refresh the service tests to share a helper similar to:

```rust
async fn bootstrap_test_pool() -> Arc<DbPool> {
    let settings = DbPoolSettings::new("sqlite::memory:");
    let pool = settings.create_pool().await.expect("sqlite pool");
    run_migrations(&pool, "sqlite::memory:").await.expect("migrations");
    Arc::new(pool)
}
```

- Apply the helper to `Services::tests`, `AppState::tests`, `auth_service.rs`
  (lines 216-520), and `refresh_token_service.rs` (lines 318-430); this keeps
  unit tests database-agnostic and ready for future Postgres matrices.

## 5. Automated test matrix & migrations

1. **Default SQLite run**
   - In `crates/board/tests/room_repository_tests.rs:8-250`, swap the direct
     `SqlitePool::connect(":memory:")` calls for
     `DbPoolSettings::new("sqlite::memory:")`. Re-use the helper from Section 4
     and ensure every test invokes
     `run_migrations(pool.as_ref(), "sqlite::memory:")` before performing
     queries.
   - Adopt the same helper inside service/unit tests to avoid duplicate
     bootstrap logic.

2. **PostgreSQL run**
   - Spin up the `postgres` service from `docker-compose.yml` (ports 5432).
     Example:

```bash
docker compose up -d postgres
export DATABASE_URL=postgres://postgres:password@localhost:5432/elizabeth
cargo test -- --ignored pg
```

- Mark Pg-only suites with `#[ignore = "requires postgres"]` (or gate them under
  `#[cfg_attr(not(feature = "pg-tests"), ignore)]`) so day-to-day `cargo test`
  stays fast on SQLite.
- When `DATABASE_URL` is a Postgres URI, the same helper should reuse
  `DbPoolSettings::new(env_url)` and run the `./migrations_pg` chain
  automatically.

3. **Migrations**
   - Keep SQLite migrations in `migrations/` and Postgres in `migrations_pg/`.
     The selection already happens in `db::run_migrations`; document this
     behavior in `README.md` and confirm
     `docs/postgres-migration-implementation.md` stays the single source of
     truth.
   - After every schema edit, regenerate `.sqlx` data for both drivers via
     `cargo sqlx prepare -- --lib --no-default-features --features sqlite` and
     the equivalent for Postgres (or drop the cache entirely per Section 7 if
     offline verification isn't needed).

## 6. Frontend HTTPS remediation & validation

### 6.1 Environment and build inputs

- `web/lib/config.ts:7-78` normalizes `NEXT_PUBLIC_API_URL` into a path-only
  `API_BASE_PATH`, while `INTERNAL_API_URL` retains the backend origin for
  server-side fetches.
- `web/next.config.mjs:12-43` rewrites `/api/v1/:path*` → `INTERNAL_API_URL`
  during runtime. Ensure `.env.docker`, `Dockerfile.frontend:35-52`, and
  `docker-compose.yml:55-70` all export `NEXT_PUBLIC_API_URL=/api/v1` and
  `INTERNAL_API_URL=http://elizabeth-backend:4092/api/v1` (or another private
  bridge).
- Follow the operational advice in `docs/https-proxy-unification.md` whenever
  production values change.

### 6.2 Build + verification steps

```bash
docker compose build --no-cache frontend
docker compose up -d frontend
```

1. Open Chrome DevTools against `https://box.yunique.top`.
2. Network tab → filter `api` and confirm every request targets
   `https://box.yunique.top/api/v1/...` with 200/401 statuses. Mixed-Content
   warnings should not appear.
3. Console tab → ensure no `ERR_MIXED_CONTENT` or `Access-Control-Allow-Origin`
   errors remain.
4. Security tab → "Main origin is secure" should stay green.
5. Record a short HAR (Network → Export) showing all API calls over HTTPS; store
   it alongside release notes.

ASCII request path:

```
Browser (HTTPS) --/api/v1--> Next.js (web/next.config.mjs rewrite)
                         │
                         ▼
             INTERNAL_API_URL (Docker bridge, HTTP)
                         │
                         ▼
                 Axum backend (AnyPool) ──► Database (Sqlite/Postgres)
```

- If any bundle still hardcodes `http://`, inspect the emitted `.next`
  artifacts, flush `web/.next`, and rebuild via `pnpm build` before re-running
  the Docker build.

## 7. Cleanup, observability, and tooling

1. **`.sqlx` cache purge**
   - Stale query descriptors live under `crates/board/.sqlx/query-*.json`.
     Delete them whenever SQL strings change so `cargo sqlx prepare` can
     regenerate driver-specific plans.
   - Suggested helper (`scripts/cleanup_sqlx.py`):

```python
#!/usr/bin/env python3
import pathlib
root = pathlib.Path(__file__).resolve().parents[1]
for json_file in root.rglob('.sqlx/*.json'):
    print(f"removing {json_file}")
    json_file.unlink()
```

2. **Dependency lockfiles**
   - Updating `crates/board/Cargo.toml` features (already includes
     `"postgres","sqlite","runtime-tokio","tls-rustls-aws-lc-rs","migrate","chrono"`)
     requires regenerating `Cargo.lock` via `cargo metadata` or `cargo check`.
     Commit the lockfile once both database test suites pass.

3. **Logging level regression**
   - `LoggingConfig` defaults to `info`
     (`crates/configrs/src/configs/app.rs:31-38`), yet `REPORT.md` notes the
     runtime always reads it as `"off"`. Add tracing around the config loader to
     print the resolved value, and verify `APP__LOGGING__LEVEL` /
     `logging.level` overrides map to the config struct before Axum initializes.

4. **Documentation cross-links**
   - Reference this file from `README.md` (database + HTTPS sections),
     `docs/postgres-migration-implementation.md`, and
     `docs/https-proxy-unification.md` so contributors discover the consolidated
     checklist quickly.

## 8. Execution order checklist

1. **Normalize Db bootstrap**: Patch `crates/board/src/lib.rs` to pass the DB
   URL into `run_migrations` and drop the nonexistent `.with_journal_mode`.
   Re-run `cargo fmt`.
2. **Rename repositories/services**: Replace the `Sqlite*` struct names
   (repositories + services) with driver-agnostic aliases and update all
   imports/tests accordingly.
3. **Fill codec gaps**: Add Postgres implementations for `ChunkStatus`,
   `UploadStatus`, and `RoomPermission`, or switch the schema to plain
   TEXT/INTEGER columns.
4. **Regenerate migrations**: Ensure any schema tweaks exist in both
   `migrations/` and `migrations_pg/`, then run `just migrate` (SQLite) followed
   by `DATABASE_URL=postgres://... just migrate` (or `cargo sqlx migrate run`).
5. **Rewire tests**: Centralize pool bootstrap helpers, gate Postgres suites
   with `#[ignore]`, and document the invocation commands.
6. **Rebuild frontend**: Run
   `docker compose build --no-cache frontend && docker compose up -d frontend`,
   then validate via DevTools as described in Section 6.
7. **Purge `.sqlx` caches**: Execute the Python helper and regenerate fresh
   metadata for whichever drivers remain enabled.
8. **Update docs**: Mention dual migrations and HTTPS verification in
   `README.md` plus link this guide from
   `docs/postgres-migration-implementation.md`.
9. **Collect logs**: Confirm `LoggingConfig.level` respects configuration
   overrides and update `REPORT.md` once the issue is reproduced and fixed.
10. **Full test matrix**: `cargo test` (SQLite) ➜
    `docker compose up -d postgres` ➜
    `DATABASE_URL=postgres://... cargo test -- --ignored pg`; capture outputs
    for release notes.

## 9. References

- `docs/postgres-migration-implementation.md` — Architectural goals for
  AnyPool + Pg support.
- `docs/https-proxy-unification.md` — Operational instructions to remove Mixed
  Content.
- `README.md` sections“数据库选择（SQLite / PostgreSQL）”and“混合内容处理” —
  User-facing documentation to update alongside code changes.
- `docker-compose.yml`, `Dockerfile.frontend`, `web/lib/config.ts`,
  `web/next.config.mjs` — Sources of truth for the proxy + env variables.
- `crates/board/src/db/mod.rs`, `crates/board/src/lib.rs`,
  `crates/board/src/services/*`, `crates/board/tests/*` — Code modules touched
  by this guide.
