# PostgreSQL/AnyPool Migration & HTTPS Hardening – Implementation Notes (2025-11-18)

## 1. Scope & Current State

- Goal: finish SQLite→PostgreSQL dual-run via `sqlx::Any`, remove mixed-content
  issues, restore automated tests (SQLite default + optional Postgres), and
  align repositories/services/tests with the new pool/type model.
- Status: Migration foundation landed; key codecs and repos are Any-ready.
  `cargo check` still fails due to missing chunk-upload repository APIs and a
  trait import; see §3.4.

## 2. Key Code Changes (with references)

- **Server bootstrap now URL-aware & journal-safe**
  - `start_server` passes `db_url` into migrations and applies SQLite
    `PRAGMA journal_mode` only when needed (`crates/board/src/lib.rs:59-118`).
  - Pool builder switched to `DbPoolSettings` without Sqlite-specific APIs
    (`crates/board/src/db/mod.rs:30-72`).
- **Cross-DB enum codecs added**
  - `ChunkStatus` implements Pg + Sqlite `Type/Decode/Encode`; shared string
    storage (`crates/board/src/models/room/chunk_upload.rs:6-93`).
  - `UploadStatus` same pattern for reservations
    (`crates/board/src/models/room/upload_reservation.rs:7-94`).
  - `RoomPermission` now encodes to Pg as `i16` bits; Sqlite unchanged
    (`crates/board/src/models/room/permission.rs:7-133`).
- **Repositories renamed to Any-first**
  - Room/refresh token/content/chunk/upload reservation repos use `Any` executor
    and Arc<DbPool>, dropping `Sqlite*` names (examples:
    `room_refresh_token_repository.rs:1-172`,
    `room_upload_reservation_repository.rs:1-220`).
- **Reservation reboot**
  - `RoomUploadReservationRepository` now supports `reserve_upload` (room size
    bookkeeping), `find_by_token`, `release_if_pending`, and numeric reservation
    IDs; updates room `current_size` on reserve/release
    (`crates/board/src/repository/room_upload_reservation_repository.rs:32-220`).
- **Auth/refresh token services & tests**
  - Tests build pools with `DbPoolSettings + run_migrations` (no direct
    `SqlitePool`) (`crates/board/src/services/auth_service.rs:213-280`,
    `crates/board/src/services/refresh_token_service.rs:320-397`).
  - Handlers use repositories instead of raw queries for room lookup
    (`crates/board/src/handlers/auth.rs:340-347`).
- **LoggingConfig behavior**
  - No code change yet; log level still parsed via `init/log_service.rs`,
    defaults to INFO. Needs runtime assertion (see §3.5 planned work).

## 3. Open Issues / Next Actions

1. **Chunk upload repository gaps** — handlers expect methods that do not exist:
   - `find_by_reservation_and_index`, `find_by_reservation_id`,
     `count_by_reservation_id`, `update_uploaded_chunks`, and `create` should
     take `&RoomChunkUpload` (currently mismatched) (handlers at
     `crates/board/src/handlers/chunked_upload.rs:380-520, 700-770`).
   - Action: rewrite `room_chunk_upload_repository.rs` to mirror schema
     `room_chunk_uploads(reservation_id, chunk_index, chunk_hash, upload_status, created_at, updated_at)`
     and expose the above helpers.
2. **Trait import for Room repo** — `handlers/auth.rs` needs
   `use crate::repository::room_repository::IRoomRepository;` or route through
   `RoomRepository::find_by_id` via the trait; currently compilation fails
   (`crates/board/src/handlers/auth.rs:340-347`).
3. **Type/Decode for Any** — `sqlx::query_as!` over `Room` with `Any` backend
   triggered missing `Type<Any>` for `NaiveDateTime/RoomPermission`; switching
   to repository-based access should remove the macro usage. Verify after fix in
   (2).
4. **Mixed-content verification & frontend rebuild** — not yet executed;
   requires
   `docker compose build --no-cache frontend && docker compose up -d frontend`
   plus DevTools HAR capture per guide.
5. **.sqlx cache cleanup** — pending. Should
   `find crates/board/.sqlx -name '*.json' -delete` after `sqlx prepare` once
   schema stabilizes.
6. **LoggingConfig runtime test** — add test covering CLI/env/file priority;
   update REPORT accordingly.

## 4. Proposed Fix Plan (pseudo-code)

```
// chunk repository redesign
struct RoomChunkUploadRepository { pool: Arc<DbPool> }
impl IRoomChunkUploadRepository for ... {
  create(upload) -> INSERT ... RETURNING id;
  find_by_reservation_and_index(res_id, idx) -> SELECT ... WHERE reservation_id=? AND chunk_index=?;
  find_by_reservation_id(res_id) -> SELECT ... ORDER BY chunk_index;
  count_by_reservation_id(res_id) -> SELECT COUNT(*);
  update_uploaded_chunks(res_id, uploaded) -> UPDATE ... SET uploaded_chunks=?;
  finalize(res_id) -> UPDATE status='verified' or 'uploaded';
}

// auth handler
use crate::repository::room_repository::IRoomRepository;
let repo = RoomRepository::new(db_pool.clone());
repo.find_by_id(room_id).await?;

// logging config test (outline)
set env LOG_LEVEL=debug; run cfg_service::init; assert cfg.app.logging.level == "debug";
```

## 5. Data/Control Flow (ASCII)

```
Browser (HTTPS) -> Next.js (/api/v1/* rewrite) -> Axum
    |                                     |
    | run_migrations(db_url)               | uses DbPool=Any (Sqlite/Postgres)
    v                                     v
Room/Upload/Token services ----> Repositories ----> Database
          ^                                  \
          |                                   +-- room_upload_reservations: reserve/release size
          |                                   +-- room_chunk_uploads: pending redesign
```

## 6. File-by-File Change Log

- `crates/board/src/lib.rs:59-118` — start_server now Any-aware; applies SQLite
  journal pragma only when DB is SQLite.
- `crates/board/src/db/mod.rs:1-72` — pool builder uses
  `AnyPoolOptions::new().connect(url)`; migrations pick path via URL.
- `crates/board/src/models/room/chunk_upload.rs:6-120` — Pg/Sqlite codecs +
  common display helper.
- `crates/board/src/models/room/upload_reservation.rs:7-94` — Pg/Sqlite codecs
  for `UploadStatus`.
- `crates/board/src/models/room/permission.rs:7-133` — Pg codecs for bitflags
  `RoomPermission`.
- `crates/board/src/repository/room_upload_reservation_repository.rs:32-220` —
  new reserve/find/release APIs with room size accounting.
- `crates/board/src/repository/room_refresh_token_repository.rs:1-172` — Any
  executor, blacklist ops consolidated.
- `crates/board/src/services/auth_service.rs:213-280` &
  `crates/board/src/services/refresh_token_service.rs:320-397` — tests use
  DbPool + migrations helper.
- `crates/board/src/handlers/auth.rs:340-347` — now delegates room lookup to
  repository (needs trait import).
- `crates/board/src/handlers/content.rs:221-279` — upload reservation flow uses
  new repo and background release.

## 7. Next Steps Checklist (execution order)

1. Rewrite `room_chunk_upload_repository.rs` and align handlers with its API;
   re-run `cargo check`.
2. Import `IRoomRepository` in `handlers/auth.rs`; prefer repository calls over
   `query_as!` macros.
3. Run `cargo check` → `cargo test` (SQLite).
4. Purge `.sqlx` caches and regenerate via `sqlx prepare` for both SQLite and
   Postgres URLs.
5. Rebuild frontend
   (`docker compose build --no-cache frontend && docker compose up -d frontend`)
   and capture DevTools HAR + mixed-content evidence.
6. Add LoggingConfig priority test; update `REPORT.md` and `README.md` links to
   guides.

---

Document generated: 2025-11-18.
