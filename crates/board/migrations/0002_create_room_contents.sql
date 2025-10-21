-- 0002_create_room_contents.sql
-- 房间内容表

CREATE TABLE IF NOT EXISTS room_contents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    content_type INTEGER NOT NULL,
    text TEXT,
    url TEXT,
    path TEXT,
    size INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

CREATE TRIGGER IF NOT EXISTS trg_room_contents_updated_at
AFTER UPDATE ON room_contents
FOR EACH ROW
BEGIN
    UPDATE room_contents SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
