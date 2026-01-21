-- ----------------------------------------------------------------------------
-- 003: Normalize datetime columns to TEXT (PostgreSQL)
-- ----------------------------------------------------------------------------
-- Rationale:
-- - The backend uses `sqlx::AnyPool` to support SQLite + PostgreSQL with one codepath.
-- - `sqlx::Any` does not implement `Type/Encode` for chrono datetime types, so the
--   application binds datetimes as strings (see `models::room::row_utils`).
-- - PostgreSQL schema in 001 used TIMESTAMPTZ, which rejects TEXT parameters without
--   explicit casts. To keep a single portable codepath, we store datetimes as TEXT
--   in PostgreSQL as well (same as SQLite).
--
-- Notes:
-- - We use a lexicographically sortable UTC format:
--   "YYYY-MM-DD HH:MM:SS.US" (microseconds, 24h).
-- - Existing data is converted to the same UTC string format.
-- ----------------------------------------------------------------------------

-- Drop views that depend on timestamp arithmetic or column types.
DROP VIEW IF EXISTS v_active_tokens;
DROP VIEW IF EXISTS v_chunked_upload_status;
DROP VIEW IF EXISTS v_room_summary;

-- Remove defaults before changing column types.
ALTER TABLE rooms ALTER COLUMN created_at DROP DEFAULT;
ALTER TABLE rooms ALTER COLUMN updated_at DROP DEFAULT;

ALTER TABLE room_contents ALTER COLUMN created_at DROP DEFAULT;
ALTER TABLE room_contents ALTER COLUMN updated_at DROP DEFAULT;

ALTER TABLE room_tokens ALTER COLUMN created_at DROP DEFAULT;

ALTER TABLE room_refresh_tokens ALTER COLUMN created_at DROP DEFAULT;

ALTER TABLE token_blacklist ALTER COLUMN created_at DROP DEFAULT;

ALTER TABLE room_upload_reservations ALTER COLUMN reserved_at DROP DEFAULT;
ALTER TABLE room_upload_reservations ALTER COLUMN created_at DROP DEFAULT;
ALTER TABLE room_upload_reservations ALTER COLUMN updated_at DROP DEFAULT;

ALTER TABLE room_chunk_uploads ALTER COLUMN created_at DROP DEFAULT;
ALTER TABLE room_chunk_uploads ALTER COLUMN updated_at DROP DEFAULT;

ALTER TABLE room_access_logs ALTER COLUMN access_time DROP DEFAULT;

-- ----------------------------------------------------------------------------
-- rooms
-- ----------------------------------------------------------------------------
ALTER TABLE rooms
  ALTER COLUMN expire_at TYPE TEXT
    USING CASE
      WHEN expire_at IS NULL THEN NULL
      ELSE to_char(expire_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US')
    END,
  ALTER COLUMN created_at TYPE TEXT
    USING to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN updated_at TYPE TEXT
    USING to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

-- room_gc fields added by 002 (TIMESTAMP)
ALTER TABLE rooms
  ALTER COLUMN empty_since TYPE TEXT
    USING CASE
      WHEN empty_since IS NULL THEN NULL
      ELSE to_char(empty_since, 'YYYY-MM-DD HH24:MI:SS.US')
    END,
  ALTER COLUMN cleanup_after TYPE TEXT
    USING CASE
      WHEN cleanup_after IS NULL THEN NULL
      ELSE to_char(cleanup_after, 'YYYY-MM-DD HH24:MI:SS.US')
    END;

-- ----------------------------------------------------------------------------
-- room_contents
-- ----------------------------------------------------------------------------
ALTER TABLE room_contents
  ALTER COLUMN created_at TYPE TEXT
    USING to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN updated_at TYPE TEXT
    USING to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

-- ----------------------------------------------------------------------------
-- room_tokens
-- ----------------------------------------------------------------------------
ALTER TABLE room_tokens
  ALTER COLUMN expires_at TYPE TEXT
    USING to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN revoked_at TYPE TEXT
    USING CASE
      WHEN revoked_at IS NULL THEN NULL
      ELSE to_char(revoked_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US')
    END,
  ALTER COLUMN created_at TYPE TEXT
    USING to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

-- ----------------------------------------------------------------------------
-- room_refresh_tokens
-- ----------------------------------------------------------------------------
ALTER TABLE room_refresh_tokens
  ALTER COLUMN expires_at TYPE TEXT
    USING to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN created_at TYPE TEXT
    USING to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN last_used_at TYPE TEXT
    USING CASE
      WHEN last_used_at IS NULL THEN NULL
      ELSE to_char(last_used_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US')
    END;

-- ----------------------------------------------------------------------------
-- token_blacklist
-- ----------------------------------------------------------------------------
ALTER TABLE token_blacklist
  ALTER COLUMN expires_at TYPE TEXT
    USING to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN created_at TYPE TEXT
    USING to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

-- ----------------------------------------------------------------------------
-- room_upload_reservations
-- ----------------------------------------------------------------------------
ALTER TABLE room_upload_reservations
  ALTER COLUMN reserved_at TYPE TEXT
    USING to_char(reserved_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN expires_at TYPE TEXT
    USING to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN consumed_at TYPE TEXT
    USING CASE
      WHEN consumed_at IS NULL THEN NULL
      ELSE to_char(consumed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US')
    END,
  ALTER COLUMN created_at TYPE TEXT
    USING to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN updated_at TYPE TEXT
    USING to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

-- ----------------------------------------------------------------------------
-- room_chunk_uploads
-- ----------------------------------------------------------------------------
ALTER TABLE room_chunk_uploads
  ALTER COLUMN created_at TYPE TEXT
    USING to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
  ALTER COLUMN updated_at TYPE TEXT
    USING to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

-- ----------------------------------------------------------------------------
-- room_access_logs
-- ----------------------------------------------------------------------------
ALTER TABLE room_access_logs
  ALTER COLUMN access_time TYPE TEXT
    USING to_char(access_time AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

-- ----------------------------------------------------------------------------
-- Restore defaults (TEXT now)
-- ----------------------------------------------------------------------------
ALTER TABLE rooms ALTER COLUMN created_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');
ALTER TABLE rooms ALTER COLUMN updated_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

ALTER TABLE room_contents ALTER COLUMN created_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');
ALTER TABLE room_contents ALTER COLUMN updated_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

ALTER TABLE room_tokens ALTER COLUMN created_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

ALTER TABLE room_refresh_tokens ALTER COLUMN created_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

ALTER TABLE token_blacklist ALTER COLUMN created_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

ALTER TABLE room_upload_reservations ALTER COLUMN reserved_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');
ALTER TABLE room_upload_reservations ALTER COLUMN created_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');
ALTER TABLE room_upload_reservations ALTER COLUMN updated_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

ALTER TABLE room_chunk_uploads ALTER COLUMN created_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');
ALTER TABLE room_chunk_uploads ALTER COLUMN updated_at SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

ALTER TABLE room_access_logs ALTER COLUMN access_time SET DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');

-- ----------------------------------------------------------------------------
-- Update trigger function for TEXT updated_at.
-- ----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ----------------------------------------------------------------------------
-- Recreate views with explicit casts for timestamp arithmetic.
-- ----------------------------------------------------------------------------
CREATE OR REPLACE VIEW v_room_summary AS
SELECT
    r.id,
    r.name,
    r.slug,
    CASE WHEN r.password IS NOT NULL THEN TRUE ELSE FALSE END as has_password,
    r.status,
    r.max_size,
    r.current_size,
    CAST(r.current_size AS NUMERIC) / r.max_size * 100 as usage_percentage,
    r.max_times_entered,
    r.current_times_entered,
    r.expire_at,
    r.created_at,
    r.updated_at,
    r.permission,
    COUNT(DISTINCT rc.id) as content_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 0 THEN rc.id END) as text_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 1 THEN rc.id END) as image_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 2 THEN rc.id END) as file_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 3 THEN rc.id END) as url_count
FROM rooms r
LEFT JOIN room_contents rc ON r.id = rc.room_id
GROUP BY r.id;

CREATE OR REPLACE VIEW v_chunked_upload_status AS
SELECT
    rur.id as reservation_id,
    rur.room_id,
    rur.token_jti,
    rur.reserved_size,
    rur.chunked_upload,
    rur.total_chunks,
    rur.uploaded_chunks,
    rur.file_hash,
    rur.chunk_size,
    rur.upload_status,
    rur.expires_at,
    rur.created_at,
    rur.updated_at,
    CASE
        WHEN rur.total_chunks IS NULL OR rur.total_chunks = 0 THEN 0.0
        ELSE CAST(rur.uploaded_chunks AS NUMERIC) / rur.total_chunks * 100
    END as upload_progress,
    COUNT(rcu.id) as total_uploaded_chunks,
    COUNT(CASE WHEN rcu.upload_status = 'uploaded' THEN 1 END) as verified_chunks,
    COUNT(CASE WHEN rcu.upload_status = 'failed' THEN 1 END) as failed_chunks,
    CASE
        WHEN rur.uploaded_chunks = 0 THEN NULL
        WHEN rur.uploaded_chunks >= rur.total_chunks THEN 0
        ELSE (rur.total_chunks - rur.uploaded_chunks) *
             EXTRACT(EPOCH FROM ((rur.updated_at)::timestamp - (rur.created_at)::timestamp)) / rur.uploaded_chunks
    END as estimated_remaining_seconds
FROM room_upload_reservations rur
LEFT JOIN room_chunk_uploads rcu ON rur.id = rcu.reservation_id
WHERE rur.chunked_upload = TRUE
GROUP BY rur.id;

CREATE OR REPLACE VIEW v_active_tokens AS
SELECT
    rt.id,
    rt.room_id,
    rt.jti,
    rt.expires_at,
    rt.created_at,
    r.name as room_name,
    r.slug as room_slug,
    r.status as room_status,
    EXTRACT(EPOCH FROM ((rt.expires_at)::timestamp - (NOW() AT TIME ZONE 'UTC'))) as seconds_until_expiry
FROM room_tokens rt
INNER JOIN rooms r ON rt.room_id = r.id
WHERE rt.revoked_at IS NULL
  AND (rt.expires_at)::timestamp > (NOW() AT TIME ZONE 'UTC');
