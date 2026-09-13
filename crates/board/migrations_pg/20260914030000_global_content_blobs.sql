-- 全局内容寻址 blob 表（issue #200 / #196）：
-- hash + owner_room_id 唯一定位一份物理对象；owner_room_id 是"首次存入的房间"，
-- 全局去重开启时其他房间的引用也计数在同一行。per-room 模式下查找按 owner 作用域。
-- 旧 room_content_blobs 按 (room_id, hash) 一一平移，引用计数不变。
CREATE TABLE content_blobs (
    id BIGSERIAL PRIMARY KEY,
    hash TEXT NOT NULL,
    owner_room_id BIGINT NOT NULL,
    locator TEXT NOT NULL,
    size BIGINT NOT NULL,
    ref_count BIGINT NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(hash, owner_room_id)
);

INSERT INTO content_blobs (hash, owner_room_id, locator, size, ref_count, created_at, updated_at)
    SELECT hash, room_id, locator, size, ref_count, created_at, updated_at
    FROM room_content_blobs;

DROP TABLE room_content_blobs;

CREATE INDEX idx_content_blobs_hash ON content_blobs(hash);
CREATE INDEX idx_content_blobs_owner ON content_blobs(owner_room_id);
