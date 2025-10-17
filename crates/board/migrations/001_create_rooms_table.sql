-- 创建房间表
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    password TEXT,
    status INTEGER NOT NULL DEFAULT 0, -- 0: Open, 1: Lock, 2: Close
    max_size INTEGER NOT NULL DEFAULT 10485760, -- 默认 10MB
    current_size INTEGER NOT NULL DEFAULT 0,
    max_times_entered INTEGER NOT NULL DEFAULT 100,
    current_times_entered INTEGER NOT NULL DEFAULT 0,
    expire_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    permission INTEGER NOT NULL DEFAULT 1 -- 默认 (1): 允许删除 (8), 允许编辑 (4), 允许下载 (2), 允许预览 (1)
);

-- 创建触发器，自动更新 updated_at 字段
CREATE TRIGGER IF NOT EXISTS update_rooms_updated_at
    AFTER UPDATE ON rooms
    FOR EACH ROW
BEGIN
    UPDATE rooms SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
