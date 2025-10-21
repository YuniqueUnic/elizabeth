-- Elizabeth 项目初始数据库架构
-- 创建时间：2024-10-21
-- 版本：1.0.0
-- 描述：合并所有迁移文件为单一初始架构文件

-- ========================================
-- 1. 房间表 (rooms) - 主表
-- ========================================
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL DEFAULT '',
    password TEXT,
    status INTEGER NOT NULL DEFAULT 0,
    max_size INTEGER NOT NULL DEFAULT 10485760,
    current_size INTEGER NOT NULL DEFAULT 0,
    max_times_entered INTEGER NOT NULL DEFAULT 100,
    current_times_entered INTEGER NOT NULL DEFAULT 0,
    expire_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    permission INTEGER NOT NULL DEFAULT 1
);

-- ========================================
-- 2. 房间内容表 (room_contents) - 依赖 rooms
-- ========================================
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

-- ========================================
-- 3. 房间访问令牌表 (room_tokens) - 依赖 rooms
-- ========================================
CREATE TABLE IF NOT EXISTS room_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    jti TEXT NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    revoked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

-- ========================================
-- 4. 上传预留表 (room_upload_reservations) - 依赖 rooms
-- ========================================
CREATE TABLE IF NOT EXISTS room_upload_reservations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    token_jti TEXT NOT NULL,
    file_manifest TEXT NOT NULL,
    reserved_size INTEGER NOT NULL,
    reserved_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    consumed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

-- ========================================
-- 5. 房间访问日志表 (room_access_logs) - 依赖 rooms
-- ========================================
CREATE TABLE IF NOT EXISTS room_access_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    access_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action INTEGER NOT NULL,
    details TEXT,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

-- ========================================
-- 6. 索引创建
-- ========================================

-- rooms 索引
CREATE INDEX IF NOT EXISTS idx_rooms_name ON rooms(name);
CREATE INDEX IF NOT EXISTS idx_rooms_status ON rooms(status);
CREATE INDEX IF NOT EXISTS idx_rooms_expire_at ON rooms(expire_at);
CREATE INDEX IF NOT EXISTS idx_rooms_created_at ON rooms(created_at);

-- room_contents 索引
CREATE INDEX IF NOT EXISTS idx_room_contents_room_id ON room_contents(room_id);
CREATE INDEX IF NOT EXISTS idx_room_contents_type ON room_contents(content_type);
CREATE INDEX IF NOT EXISTS idx_room_contents_created_at ON room_contents(created_at);

-- room_access_logs 索引
CREATE INDEX IF NOT EXISTS idx_room_access_logs_room_id ON room_access_logs(room_id);
CREATE INDEX IF NOT EXISTS idx_room_access_logs_access_time ON room_access_logs(access_time);
CREATE INDEX IF NOT EXISTS idx_room_access_logs_action ON room_access_logs(action);

-- room_tokens 索引
CREATE INDEX IF NOT EXISTS idx_room_tokens_room_id ON room_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_tokens_expires_at ON room_tokens(expires_at);

-- room_upload_reservations 索引
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_room_id
    ON room_upload_reservations(room_id);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_token_jti
    ON room_upload_reservations(token_jti);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_expires_at
    ON room_upload_reservations(expires_at);

-- ========================================
-- 7. 触发器创建
-- ========================================

-- rooms 表 updated_at 触发器
CREATE TRIGGER IF NOT EXISTS trg_rooms_updated_at
AFTER UPDATE ON rooms
FOR EACH ROW
BEGIN
    UPDATE rooms SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- room_contents 表 updated_at 触发器
CREATE TRIGGER IF NOT EXISTS trg_room_contents_updated_at
AFTER UPDATE ON room_contents
FOR EACH ROW
BEGIN
    UPDATE room_contents SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- room_upload_reservations 表 updated_at 触发器
CREATE TRIGGER IF NOT EXISTS trg_room_upload_reservations_updated_at
AFTER UPDATE ON room_upload_reservations
FOR EACH ROW
BEGIN
    UPDATE room_upload_reservations
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;
