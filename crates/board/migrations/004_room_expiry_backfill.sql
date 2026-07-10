-- Backfill explicit expiry for legacy rooms created before room expiry defaults existed.
--
-- Historical UI promised a one-week expiry even when `expire_at` stayed NULL.
-- Make the persisted policy explicit so request-time enforcement and background
-- lifecycle cleanup behave consistently after upgrade.

UPDATE rooms
SET expire_at = datetime(COALESCE(created_at, CURRENT_TIMESTAMP), '+7 days')
WHERE expire_at IS NULL;

ALTER TABLE room_upload_reservations
    ADD COLUMN owner_token_jti TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_rooms_expiry_due
    ON rooms(expire_at)
    WHERE expire_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_upload_reservations_owner
    ON room_upload_reservations(room_id, owner_token_jti);
