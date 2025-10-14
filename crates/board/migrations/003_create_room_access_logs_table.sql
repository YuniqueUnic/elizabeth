-- 创建房间访问日志表
CREATE TABLE IF NOT EXISTS room_access_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    access_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action INTEGER NOT NULL, -- 0: enter, 1: exit, 2: create_content, 3: delete_content
    details TEXT,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);
