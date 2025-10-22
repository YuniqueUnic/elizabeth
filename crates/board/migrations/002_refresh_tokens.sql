-- Elizabeth 项目令牌刷新机制数据库迁移
-- 创建时间：2025-10-22
-- 版本：2.0.0 (简化版)
-- 描述：添加刷新令牌支持和令牌黑名单机制

-- ========================================
-- 1. 房间刷新令牌表 (room_refresh_tokens)
-- ========================================
CREATE TABLE IF NOT EXISTS room_refresh_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    access_token_jti TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
);

-- ========================================
-- 2. 令牌黑名单表 (token_blacklist)
-- ========================================
CREATE TABLE IF NOT EXISTS token_blacklist (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    jti TEXT NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 3. 索引创建
-- ========================================

-- room_refresh_tokens 索引
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_room_id
    ON room_refresh_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_access_jti
    ON room_refresh_tokens(access_token_jti);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_token_hash
    ON room_refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_expires_at
    ON room_refresh_tokens(expires_at);

-- token_blacklist 索引
CREATE INDEX IF NOT EXISTS idx_token_blacklist_jti
    ON token_blacklist(jti);
CREATE INDEX IF NOT EXISTS idx_token_blacklist_expires_at
    ON token_blacklist(expires_at);
