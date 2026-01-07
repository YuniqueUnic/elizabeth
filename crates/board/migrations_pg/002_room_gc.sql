-- ----------------------------------------------------------------------------
-- 002: Room GC fields (PostgreSQL)
-- ----------------------------------------------------------------------------
-- See crates/board/migrations/002_room_gc.sql for rationale.
-- ----------------------------------------------------------------------------

ALTER TABLE rooms ADD COLUMN IF NOT EXISTS empty_since TIMESTAMP;
ALTER TABLE rooms ADD COLUMN IF NOT EXISTS cleanup_after TIMESTAMP;

CREATE INDEX IF NOT EXISTS idx_rooms_cleanup_after ON rooms(cleanup_after);
CREATE INDEX IF NOT EXISTS idx_rooms_empty_since ON rooms(empty_since);
