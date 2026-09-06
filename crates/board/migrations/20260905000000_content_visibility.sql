-- ============================================================================
-- 内容可见性：room_contents.hidden（msg/file.visibility.manage 能力控制）
-- ============================================================================

ALTER TABLE room_contents ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_room_contents_room_hidden
    ON room_contents (room_id, hidden);
