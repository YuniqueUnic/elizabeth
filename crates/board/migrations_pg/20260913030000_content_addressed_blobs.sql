-- 内容寻址存储与引用计数（issue #200）：
-- room_contents.hash 记录内容 SHA-256（存量行与 presigned 直传为 NULL，删除走旧路径）；
-- room_content_blobs 以 (room_id, hash) 唯一，locator 指向物理对象，ref_count 为引用计数。
ALTER TABLE room_contents ADD COLUMN hash TEXT;

CREATE TABLE room_content_blobs (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL,
    hash TEXT NOT NULL,
    locator TEXT NOT NULL,
    size BIGINT NOT NULL,
    ref_count BIGINT NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(room_id, hash)
);

CREATE INDEX idx_room_content_blobs_room ON room_content_blobs(room_id);
