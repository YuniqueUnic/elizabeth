-- ----------------------------------------------------------------------------
-- 002: Room GC fields
-- ----------------------------------------------------------------------------
-- Purpose:
--   Track when a room becomes empty (no active WebSocket connections) and schedule
--   a delayed cleanup for rooms that are "full" (max_times_entered reached) and
--   have no explicit expiration (expire_at IS NULL).
--
-- Columns:
--   empty_since   - When the last connection left the room (NULL = active/unknown)
--   cleanup_after - When the room is eligible for purge (NULL = not scheduled)
-- ----------------------------------------------------------------------------

ALTER TABLE rooms ADD COLUMN empty_since DATETIME;
ALTER TABLE rooms ADD COLUMN cleanup_after DATETIME;

CREATE INDEX IF NOT EXISTS idx_rooms_cleanup_after ON rooms(cleanup_after);
CREATE INDEX IF NOT EXISTS idx_rooms_empty_since ON rooms(empty_since);
