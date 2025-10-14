-- 创建房间内容表
CREATE TABLE IF NOT EXISTS room_contents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    content_type INTEGER NOT NULL, -- 0: text, 1: image, 2: file
    content_data TEXT NOT NULL,
    file_name TEXT,
    file_size INTEGER,
    file_path TEXT,
    mime_type TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

-- 创建触发器，自动更新 updated_at 字段
CREATE TRIGGER IF NOT EXISTS update_room_contents_updated_at
    AFTER UPDATE ON room_contents
    FOR EACH ROW
BEGIN
    UPDATE room_contents SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
