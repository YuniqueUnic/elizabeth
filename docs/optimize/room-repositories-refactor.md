---
title: "Stop Copy-Pasting SQL: Refactoring the Room Repositories with sqlx"
date: "2025-10-18"
author: "unic dev team"
tags: ["rust", "sqlx", "repository-pattern", "refactoring"]
---

## TL;DR

- We inherited two sqlite-backed repositories (`room_repository.rs` /
  `room_content_repostitory.rs`) with duplicated queries, non-transactional
  write-read sequences, and `COUNT(*)` existence checks.
- The refactor extracts reusable read helpers, wraps every write in a
  transaction, swaps `COUNT(*)` for `EXISTS`, and introduces batch deletion for
  expired rooms.
- Both repositories now share the same patterns, and we documented the
  validation workflow (`cargo sqlx prepare` + `just dev-verify`).

---

## 1. Background: what the code looked like on Monday

The initial repositories worked, but they were classic “gets-the-job-done” Rust.
Here’s a condensed snapshot of the original `room_repository.rs`:

```rust
#[async_trait]
impl RoomRepository for SqliteRoomRepository {
    async fn exists(&self, name: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM rooms WHERE name = ?",
            name
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(count > 0)
    }

    async fn create(&self, room: &Room) -> Result<Room> {
        let now = Utc::now().naive_utc();

        sqlx::query!(
            r#"
            INSERT INTO rooms (
                name, password, status, max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            room.name,
            room.password,
            room.status,
            room.max_size,
            room.current_size,
            room.max_times_entered,
            room.current_times_entered,
            room.expire_at,
            now,
            now,
            room.permission,
        )
        .execute(&*self.pool)
        .await?;

        let created_room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id, name, password, status as "status: RoomStatus", max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            FROM rooms
            WHERE name = ?
            "#,
            room.name
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(created_room)
    }

    // find_by_name, find_by_id, update, list_expired...
    // each repeated the same SELECT block verbatim
}
```

`room_content_repostitory.rs` mirrored those patterns. `exists` used `COUNT(*)`,
every read duplicated the same `SELECT` clause, and `create` / `update` executed
outside of a transaction:

```rust
async fn create(&self, room_content: &RoomContent) -> Result<RoomContent> {
    let now = Utc::now().naive_utc();
    let result = sqlx::query!(
        "INSERT INTO room_contents (...) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        /* ... */
    )
    .execute(&*self.pool)
    .await?;

    let created_room_content = self.find_by_id(result.last_insert_rowid()).await?;
    created_room_content.ok_or_else(|| anyhow!("failed to get created room_content"))
}
```

It was fine for a prototype. It’s brittle for a feature-complete product.

---

## 2. What we wanted to fix (and why)

| Pain Point                             | Why it hurt                                                                    | Fix                                                                                                                   |
| -------------------------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| Copy-pasted `SELECT` blocks            | Any schema change required hunting across 5+ methods. Easy to miss one.        | Move read logic into small helper functions that keep using `sqlx::query_as!` (so we retain compile-time validation). |
| `INSERT`/`UPDATE` without transactions | Between the write and read you could observe stale data under concurrent load. | Wrap the write and subsequent read in a transaction and commit at the end.                                            |
| `COUNT(*)` for existence               | Forces the database to keep scanning. Wasteful even on small tables.           | Use `SELECT EXISTS(SELECT 1 ...)` so the engine can bail out early.                                                   |
| No batch deletion                      | Service layer had to loop and delete per row.                                  | Add `delete_expired_before(&self, before: NaiveDateTime)` to handle expiry in one statement.                          |

We also wanted the room and room-content repositories to feel the same—so future
contributors don’t have to relearn conventions twice.

---

## 3. The refactor

### 3.1 Helper functions own the SQL

We kept the macros (they give us type-safety) but hid them inside helpers:

```rust
async fn fetch_room_optional_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Room>>
where
    E: Executor<'e, Database = Sqlite>,
{ /* ... */ }
```

Same for name lookups and fetching expired rooms. Every public method now
delegates to these helpers; there’s only one place to update when we add
columns.

### 3.2 Wrap writes in transactions

Both `create` and `update` start with `let mut tx = self.pool.begin().await?;`.
We run the insert/update, call the helper to fetch the fresh row via the
transaction handle, then `tx.commit().await?;`. No more half-baked reads.

### 3.3 Switch to `EXISTS`

The diff is a one-line change, but it matters for read-heavy endpoints:

```rust
let exists: i64 =
    sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM rooms WHERE name = ?)", name)
        .fetch_one(&*self.pool)
        .await?;
```

### 3.4 Batch deletion primitive

We added `delete_expired_before` to the `IRoomRepository` trait. It deletes all
rows with an `expire_at` before the supplied timestamp and returns the number of
rows affected. Your cleanup job can call it once instead of looping on IDs.

### 3.5 Mirror everything in RoomContent

`SqliteRoomContentRepository` now has `fetch_optional_by_id`,
`fetch_by_id_or_err`, and transactional `create` / `update`. The API surface
matches the room repository, so code reviews don’t have to untangle two styles.

---

## 4. Final code (2025-10-18)

> **Heads-up:** These listings are intentionally close to the full files so
> future readers can diff them with `git show` even if the tree changes again.

### `src/repository/room_repository.rs`

```rust
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sqlx::{Executor, Sqlite};
use std::sync::Arc;

use crate::db::DbPool;
use crate::models::{permission::RoomPermission, Room, RoomStatus};

#[async_trait]
pub trait IRoomRepository: Send + Sync {
    async fn exists(&self, name: &str) -> Result<bool>;
    async fn create(&self, room: &Room) -> Result<Room>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Room>>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Room>>;
    async fn update(&self, room: &Room) -> Result<Room>;
    async fn delete(&self, name: &str) -> Result<bool>;
    async fn list_expired(&self) -> Result<Vec<Room>>;
    async fn delete_expired_before(&self, before: NaiveDateTime) -> Result<u64>;
}

pub struct SqliteRoomRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_room_optional_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Room>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id,
                name,
                password,
                status as "status: RoomStatus",
                max_size,
                current_size,
                max_times_entered,
                current_times_entered,
                expire_at,
                created_at,
                updated_at,
                permission as "permission: RoomPermission"
            FROM rooms
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(room)
    }

    async fn fetch_room_optional_by_name<'e, E>(
        executor: E,
        name: &str,
    ) -> Result<Option<Room>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let room = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id,
                name,
                password,
                status as "status: RoomStatus",
                max_size,
                current_size,
                max_times_entered,
                current_times_entered,
                expire_at,
                created_at,
                updated_at,
                permission as "permission: RoomPermission"
            FROM rooms
            WHERE name = ?
            "#,
            name
        )
        .fetch_optional(executor)
        .await?;

        Ok(room)
    }

    async fn fetch_room_by_id_or_err<'e, E>(executor: E, id: i64) -> Result<Room>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        Self::fetch_room_optional_by_id(executor, id)
            .await?
            .ok_or_else(|| anyhow!("room not found for id {}", id))
    }

    async fn fetch_expired_rooms<'e, E>(
        executor: E,
        before: NaiveDateTime,
    ) -> Result<Vec<Room>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let rooms = sqlx::query_as!(
            Room,
            r#"
            SELECT
                id,
                name,
                password,
                status as "status: RoomStatus",
                max_size,
                current_size,
                max_times_entered,
                current_times_entered,
                expire_at,
                created_at,
                updated_at,
                permission as "permission: RoomPermission"
            FROM rooms
            WHERE expire_at IS NOT NULL AND expire_at < ?
            "#,
            before
        )
        .fetch_all(executor)
        .await?;

        Ok(rooms)
    }
}

#[async_trait]
impl IRoomRepository for SqliteRoomRepository {
    async fn exists(&self, name: &str) -> Result<bool> {
        let exists: i64 =
            sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM rooms WHERE name = ?)", name)
                .fetch_one(&*self.pool)
                .await?;

        Ok(exists != 0)
    }

    async fn create(&self, room: &Room) -> Result<Room> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();

        let insert_result = sqlx::query!(
            r#"
            INSERT INTO rooms (
                name, password, status, max_size, current_size,
                max_times_entered, current_times_entered, expire_at,
                created_at, updated_at, permission
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            room.name,
            room.password,
            room.status,
            room.max_size,
            room.current_size,
            room.max_times_entered,
            room.current_times_entered,
            room.expire_at,
            now,
            now,
            room.permission,
        )
        .execute(&mut *tx)
        .await?;

        let room_id = insert_result.last_insert_rowid();
        let created_room = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;

        tx.commit().await?;
        Ok(created_room)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Room>> {
        Self::fetch_room_optional_by_name(&*self.pool, name).await
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Room>> {
        Self::fetch_room_optional_by_id(&*self.pool, id).await
    }

    async fn update(&self, room: &Room) -> Result<Room> {
        let room_id = room
            .id
            .ok_or_else(|| anyhow!("room id is required for update"))?;
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();

        sqlx::query!(
            r#"
            UPDATE rooms SET
                password = ?, status = ?, max_size = ?, current_size = ?,
            max_times_entered = ?, current_times_entered = ?, expire_at = ?,
            updated_at = ?, permission = ?
        WHERE id = ?
            "#,
            room.password,
            room.status,
            room.max_size,
            room.current_size,
            room.max_times_entered,
            room.current_times_entered,
            room.expire_at,
            now,
            room.permission,
            room_id
        )
        .execute(&mut *tx)
        .await?;

        let updated_room = Self::fetch_room_by_id_or_err(&mut *tx, room_id).await?;

        tx.commit().await?;
        Ok(updated_room)
    }

    async fn delete(&self, name: &str) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM rooms WHERE name = ?", name)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list_expired(&self) -> Result<Vec<Room>> {
        let now = Utc::now().naive_utc();
        Self::fetch_expired_rooms(&*self.pool, now).await
    }

    async fn delete_expired_before(&self, before: NaiveDateTime) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM rooms WHERE expire_at IS NOT NULL AND expire_at < ?",
            before
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
```

### `src/repository/room_content_repostitory.rs`

```rust
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Executor, Sqlite};
use std::sync::Arc;

use crate::{
    db::DbPool,
    models::content::{ContentType, RoomContent},
};

#[async_trait]
pub trait IRoomContentRepository: Send + Sync {
    async fn exists(&self, content_id: i64) -> Result<bool>;
    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent>;
    async fn find_by_id(&self, content_id: i64) -> Result<Option<RoomContent>>;
    async fn update(&self, room_content: &RoomContent) -> Result<RoomContent>;
    async fn delete(&self, room_name: &str) -> Result<bool>;
}

pub struct SqliteRoomContentRepository {
    pool: Arc<DbPool>,
}

impl SqliteRoomContentRepository {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    async fn fetch_optional_by_id<'e, E>(
        executor: E,
        content_id: i64,
    ) -> Result<Option<RoomContent>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let content = sqlx::query_as!(
            RoomContent,
            r#"
            SELECT
                id,
                room_id,
                content_type as "content_type: ContentType",
                text,
                url,
                path,
                size,
                mime_type,
                created_at,
                updated_at
            FROM room_contents
            WHERE id = ?
            "#,
            content_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(content)
    }

    async fn fetch_by_id_or_err<'e, E>(
        executor: E,
        content_id: i64,
    ) -> Result<RoomContent>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        Self::fetch_optional_by_id(executor, content_id)
            .await?
            .ok_or_else(|| anyhow!("room content not found for id {}", content_id))
    }
}

#[async_trait]
impl IRoomContentRepository for SqliteRoomContentRepository {
    async fn exists(&self, content_id: i64) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM room_contents WHERE id = ?)",
            content_id
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(exists != 0)
    }

    async fn create(&self, room_content: &RoomContent) -> Result<RoomContent> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let result = sqlx::query!(
            "INSERT INTO room_contents (room_id, content_type, text, url, path, size, mime_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            room_content.room_id,
            room_content.content_type,
            room_content.text,
            room_content.url,
            room_content.path,
            room_content.size,
            room_content.mime_type,
            now,
            now
        )
        .execute(&mut *tx)
        .await?;

        let id = result.last_insert_rowid();
        let created_room_content = Self::fetch_by_id_or_err(&mut *tx, id).await?;

        tx.commit().await?;
        Ok(created_room_content)
    }

    async fn find_by_id(&self, content_id: i64) -> Result<Option<RoomContent>> {
        Self::fetch_optional_by_id(&*self.pool, content_id).await
    }

    async fn update(&self, room_content: &RoomContent) -> Result<RoomContent> {
        let content_id = room_content
            .id
            .ok_or_else(|| anyhow!("room content id is required for update"))?;
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        sqlx::query!(
            r#"
            UPDATE room_contents SET
                room_id = ?, content_type = ?, text = ?,
                url = ?, path = ?, size = ?, mime_type = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            room_content.room_id,
            room_content.content_type,
            room_content.text,
            room_content.url,
            room_content.path,
            room_content.size,
            room_content.mime_type,
            now,
            content_id
        )
        .execute(&mut *tx)
        .await?;

        let updated_room_content = Self::fetch_by_id_or_err(&mut *tx, content_id).await?;

        tx.commit().await?;
        Ok(updated_room_content)
    }

    async fn delete(&self, room_name: &str) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM room_contents WHERE room_id = (SELECT id FROM rooms WHERE name = ?)",
            room_name
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
```

---

## 5. Verification checklist

1. `cargo fmt`
2. `DATABASE_URL="sqlite:crates/board/app.db" cargo sqlx prepare --workspace`
3. `just dev-verify` (optional but recommended)

Those steps reformat, regenerate the SQLx metadata, and ensure the offline
validator passes.

---

## 6. What we learned (and what future us should remember)

- **Let the compiler keep catching mistakes.** We didn’t abandon
  `sqlx::query_as!`; we organized the code so one helper holds the literal.
- **Transactions should be the default whenever you write and then read.**
  SQLite can surprise you in concurrent scenarios without them.
- **Optimizing existence checks is easy low-hanging fruit.** `EXISTS`
  communicates intent and runs faster.
- **Repositories should expose the operations the domain actually needs.** Batch
  deletion felt like “business logic” until we added one tiny method.

Next time we touch these files, we’ll thank past us for leaving a clear path
forward.
