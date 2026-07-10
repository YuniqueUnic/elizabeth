-- Backfill explicit expiry for legacy rooms created before room expiry defaults existed.
--
-- Historical UI promised a one-week expiry even when `expire_at` stayed NULL.
-- Make the persisted policy explicit so request-time enforcement and background
-- lifecycle cleanup behave consistently after upgrade.

UPDATE rooms
SET expire_at = to_char(
        COALESCE(created_at::timestamp, NOW() AT TIME ZONE 'UTC') + INTERVAL '7 days',
        'YYYY-MM-DD HH24:MI:SS.US'
    ),
    updated_at = to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US')
WHERE expire_at IS NULL;

ALTER TABLE room_upload_reservations
    ADD COLUMN IF NOT EXISTS owner_token_jti TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_rooms_expiry_due
    ON rooms(expire_at)
    WHERE expire_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_upload_reservations_owner
    ON room_upload_reservations(room_id, owner_token_jti);
