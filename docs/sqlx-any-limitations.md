# SQLx Any Driver Notes

## 1. Data Type Coverage

`sqlx::Any` exposes only a very small set of physical column kinds (`Null`,
`Bool`, `SmallInt`, `Integer`, `BigInt`, `Real`, `Double`, `Text`, `Blob`). The
official API docs for
[`AnyTypeInfoKind`](https://docs.rs/sqlx/latest/sqlx/any/enum.AnyTypeInfoKind.html)
list these variants explicitly, which means higher-level types such as
`TIMESTAMP`/`DATETIME` are not decoded natively.citeturn1fetch0

## 2. Driver Behavior

The runtime-generic driver (`sqlx::Any`) simply multiplexes whichever concrete
driver matches the `DATABASE_URL`, but values surface through the limited kinds
shown above. SQLx’s own documentation warns that `AnyConnection`/`AnyPool`
should only be used after calling `install_default_drivers`, underscoring that
this driver is intentionally thin and experimental.citeturn1fetch2

## 3. Practical Implications

- Chrono types (`NaiveDateTime`, `DateTime<Utc>`, etc.) do not implement
  `sqlx::Type<Any>` / `Decode<Any>` because `Any` lacks dedicated timestamp
  metadata. Attempting to derive `FromRow` for structs that contain chrono
  fields will therefore fail to compile when using `AnyPool`.
- When we still need chrono values in the domain model, the safe workaround is
  to:
  1. Read/write timestamps as ISO 8601 strings (or Unix epoch integers) at the
     SQL boundary.
  2. Convert them to `NaiveDateTime` inside Rust helper functions.
  3. Keep conversions isolated so that repositories stay KISS-compliant and
     cross-database friendly.

These notes justify the manual string-based timestamp conversions introduced in
the chunk-upload repository refactor.
