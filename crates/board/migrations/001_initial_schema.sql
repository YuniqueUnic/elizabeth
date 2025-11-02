-- ============================================================================
-- Elizabeth 项目数据库架构
-- ============================================================================
-- 项目：Elizabeth - 文件分享与协作平台
-- 版本：1.0.0
-- 创建时间：2025-11-02
-- 描述：完整的数据库架构定义，包含所有表、索引、触发器和视图
--
-- 核心设计理念：
-- 1. 以房间为中心 (Room-centric)
-- 2. 无用户系统 (User-less)
-- 3. JWT 令牌认证
-- 4. 支持文件分块上传和断点续传
-- 5. 完整的审计日志
-- ============================================================================

-- ============================================================================
-- 表结构定义
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 房间表 (rooms)
-- ----------------------------------------------------------------------------
-- 描述：存储房间的基本信息和配置
-- 核心字段：
--   - name: 房间唯一名称（用户可见）
--   - slug: URL 友好的房间标识符
--   - password: 可选的房间密码（加密存储）
--   - status: 房间状态（0=开放，1=锁定，2=关闭）
--   - max_size: 房间最大存储容量（字节）
--   - current_size: 当前已使用容量（字节）
--   - max_times_entered: 最大访问次数
--   - current_times_entered: 当前访问次数
--   - expire_at: 房间过期时间
--   - permission: 房间权限位标志
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    password TEXT,
    status INTEGER NOT NULL DEFAULT 0 CHECK (status IN (0, 1, 2)),
    max_size INTEGER NOT NULL DEFAULT 10485760 CHECK (max_size > 0),
    current_size INTEGER NOT NULL DEFAULT 0 CHECK (current_size >= 0),
    max_times_entered INTEGER NOT NULL DEFAULT 100 CHECK (max_times_entered > 0),
    current_times_entered INTEGER NOT NULL DEFAULT 0 CHECK (current_times_entered >= 0),
    expire_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    permission INTEGER NOT NULL DEFAULT 1 CHECK (permission >= 0),

    -- 约束：当前容量不能超过最大容量
    CHECK (current_size <= max_size),
    -- 约束：当前访问次数不能超过最大访问次数
    CHECK (current_times_entered <= max_times_entered)
);

-- ----------------------------------------------------------------------------
-- 2. 房间内容表 (room_contents)
-- ----------------------------------------------------------------------------
-- 描述：存储房间内的所有内容（文本、图片、文件、URL）
-- 核心字段：
--   - content_type: 内容类型（0=文本，1=图片，2=文件，3=URL）
--   - text: 文本内容（用于 content_type=0）
--   - url: URL 地址（用于 content_type=3）
--   - path: 文件存储路径（UUID-based，用于 content_type=1,2）
--   - file_name: 原始文件名（用于显示和下载）
--   - size: 内容大小（字节）
--   - mime_type: MIME 类型
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_contents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    content_type INTEGER NOT NULL CHECK (content_type IN (0, 1, 2, 3)),
    text TEXT,
    url TEXT,
    path TEXT,
    file_name TEXT,
    size INTEGER DEFAULT 0 CHECK (size >= 0),
    mime_type TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE,

    -- 约束：文本类型必须有 text 字段
    CHECK (content_type != 0 OR text IS NOT NULL),
    -- 约束：URL 类型必须有 url 字段
    CHECK (content_type != 3 OR url IS NOT NULL),
    -- 约束：文件类型必须有 path 和 file_name 字段
    CHECK (content_type NOT IN (1, 2) OR (path IS NOT NULL AND file_name IS NOT NULL))
);

-- ----------------------------------------------------------------------------
-- 3. 房间访问令牌表 (room_tokens)
-- ----------------------------------------------------------------------------
-- 描述：存储 JWT 访问令牌的元数据，用于令牌验证和撤销
-- 核心字段：
--   - jti: JWT 唯一标识符
--   - expires_at: 令牌过期时间
--   - revoked_at: 令牌撤销时间（NULL 表示未撤销）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    jti TEXT NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    revoked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE,

    -- 约束：过期时间必须在创建时间之后
    CHECK (expires_at > created_at),
    -- 约束：撤销时间必须在创建时间之后
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

-- ----------------------------------------------------------------------------
-- 4. 房间刷新令牌表 (room_refresh_tokens)
-- ----------------------------------------------------------------------------
-- 描述：存储 JWT 刷新令牌的哈希值，用于令牌刷新机制
-- 核心字段：
--   - access_token_jti: 关联的访问令牌 JTI
--   - token_hash: 刷新令牌的 SHA-256 哈希值（不存储明文）
--   - last_used_at: 最后使用时间
--   - is_revoked: 是否已撤销
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_refresh_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    access_token_jti TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,

    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE,

    -- 约束：过期时间必须在创建时间之后
    CHECK (expires_at > created_at),
    -- 约束：最后使用时间必须在创建时间之后
    CHECK (last_used_at IS NULL OR last_used_at >= created_at)
);

-- ----------------------------------------------------------------------------
-- 5. 令牌黑名单表 (token_blacklist)
-- ----------------------------------------------------------------------------
-- 描述：存储已撤销的令牌 JTI，用于快速验证令牌是否有效
-- 核心字段：
--   - jti: 令牌唯一标识符
--   - expires_at: 黑名单记录过期时间（与令牌过期时间一致）
-- 注意：定期清理过期的黑名单记录以优化性能
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS token_blacklist (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    jti TEXT NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- 约束：过期时间必须在创建时间之后
    CHECK (expires_at > created_at)
);

-- ----------------------------------------------------------------------------
-- 6. 上传预留表 (room_upload_reservations)
-- ----------------------------------------------------------------------------
-- 描述：存储文件上传预留信息，支持普通上传和分块上传
-- 核心字段：
--   - token_jti: 关联的令牌 JTI
--   - file_manifest: 文件清单（JSON 格式）
--   - reserved_size: 预留的存储空间（字节）
--   - expires_at: 预留过期时间
--   - consumed_at: 预留消费时间（NULL 表示未消费）
--   - chunked_upload: 是否为分块上传
--   - total_chunks: 总分块数
--   - uploaded_chunks: 已上传分块数
--   - file_hash: 文件完整哈希值
--   - chunk_size: 分块大小
--   - upload_status: 上传状态（pending/uploading/completed/failed/expired）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_upload_reservations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    token_jti TEXT NOT NULL,
    file_manifest TEXT NOT NULL,
    reserved_size INTEGER NOT NULL CHECK (reserved_size > 0),
    reserved_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    consumed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- 分块上传相关字段
    chunked_upload BOOLEAN DEFAULT FALSE,
    total_chunks INTEGER CHECK (total_chunks IS NULL OR total_chunks > 0),
    uploaded_chunks INTEGER DEFAULT 0 CHECK (uploaded_chunks >= 0),
    file_hash TEXT,
    chunk_size INTEGER CHECK (chunk_size IS NULL OR chunk_size > 0),
    upload_status TEXT DEFAULT 'pending' CHECK (upload_status IN ('pending', 'uploading', 'completed', 'failed', 'expired')),

    FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE,

    -- 约束：过期时间必须在预留时间之后
    CHECK (expires_at > reserved_at),
    -- 约束：消费时间必须在预留时间之后
    CHECK (consumed_at IS NULL OR consumed_at >= reserved_at),
    -- 约束：已上传分块数不能超过总分块数
    CHECK (uploaded_chunks IS NULL OR total_chunks IS NULL OR uploaded_chunks <= total_chunks),
    -- 约束：分块上传必须有 total_chunks 和 chunk_size
    CHECK (chunked_upload = FALSE OR (total_chunks IS NOT NULL AND chunk_size IS NOT NULL))
);

-- ----------------------------------------------------------------------------
-- 7. 房间分块上传表 (room_chunk_uploads)
-- ----------------------------------------------------------------------------
-- 描述：存储分块上传的每个分块的详细信息
-- 核心字段：
--   - reservation_id: 关联的预留 ID
--   - chunk_index: 分块索引（从 0 开始）
--   - chunk_size: 分块大小（字节）
--   - chunk_hash: 分块哈希值（用于验证）
--   - upload_status: 分块状态（pending/uploading/uploaded/failed）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_chunk_uploads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reservation_id INTEGER NOT NULL,
    chunk_index INTEGER NOT NULL CHECK (chunk_index >= 0),
    chunk_size INTEGER NOT NULL CHECK (chunk_size > 0),
    chunk_hash TEXT,
    upload_status TEXT NOT NULL DEFAULT 'pending' CHECK (upload_status IN ('pending', 'uploading', 'uploaded', 'failed')),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (reservation_id) REFERENCES room_upload_reservations (id) ON DELETE CASCADE,

    -- 约束：同一预留的分块索引必须唯一
    UNIQUE(reservation_id, chunk_index)
);

-- ----------------------------------------------------------------------------
-- 8. 房间访问日志表 (room_access_logs)
-- ----------------------------------------------------------------------------
-- 描述：记录房间的所有访问和操作日志，用于审计和分析
-- 核心字段：
--   - ip_address: 访问者 IP 地址
--   - user_agent: 访问者 User-Agent
--   - action: 操作类型（枚举值）
--   - details: 操作详情（JSON 格式）
-- ----------------------------------------------------------------------------
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

-- ============================================================================
-- 索引定义
-- ============================================================================
-- 说明：索引用于优化查询性能，根据实际查询模式设计
-- 原则：
-- 1. 为外键创建索引（提升 JOIN 性能）
-- 2. 为常用查询条件创建索引
-- 3. 为唯一约束创建索引
-- 4. 避免过度索引（影响写入性能）
-- ============================================================================

-- ----------------------------------------------------------------------------
-- rooms 表索引
-- ----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_rooms_name ON rooms(name);
CREATE INDEX IF NOT EXISTS idx_rooms_slug ON rooms(slug);
CREATE INDEX IF NOT EXISTS idx_rooms_status ON rooms(status);
CREATE INDEX IF NOT EXISTS idx_rooms_expire_at ON rooms(expire_at);
CREATE INDEX IF NOT EXISTS idx_rooms_created_at ON rooms(created_at);
-- 复合索引：用于查询特定状态且未过期的房间
CREATE INDEX IF NOT EXISTS idx_rooms_status_expire ON rooms(status, expire_at);

-- ----------------------------------------------------------------------------
-- room_contents 表索引
-- ----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_room_contents_room_id ON room_contents(room_id);
CREATE INDEX IF NOT EXISTS idx_room_contents_content_type ON room_contents(content_type);
CREATE INDEX IF NOT EXISTS idx_room_contents_created_at ON room_contents(created_at);
-- 复合索引：用于查询特定房间的特定类型内容
CREATE INDEX IF NOT EXISTS idx_room_contents_room_type ON room_contents(room_id, content_type);

-- ----------------------------------------------------------------------------
-- room_tokens 表索引
-- ----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_room_tokens_room_id ON room_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_tokens_jti ON room_tokens(jti);
CREATE INDEX IF NOT EXISTS idx_room_tokens_expires_at ON room_tokens(expires_at);
-- 复合索引：用于查询未撤销且未过期的令牌
CREATE INDEX IF NOT EXISTS idx_room_tokens_active ON room_tokens(jti, expires_at, revoked_at);

-- ----------------------------------------------------------------------------
-- room_refresh_tokens 表索引
-- ----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_room_id ON room_refresh_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_access_jti ON room_refresh_tokens(access_token_jti);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_token_hash ON room_refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_expires_at ON room_refresh_tokens(expires_at);
-- 复合索引：用于查询有效的刷新令牌
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_active ON room_refresh_tokens(token_hash, is_revoked, expires_at);

-- ----------------------------------------------------------------------------
-- token_blacklist 表索引
-- ----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_token_blacklist_jti ON token_blacklist(jti);
CREATE INDEX IF NOT EXISTS idx_token_blacklist_expires_at ON token_blacklist(expires_at);

-- ----------------------------------------------------------------------------
-- room_upload_reservations 表索引
-- ----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_room_id ON room_upload_reservations(room_id);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_token_jti ON room_upload_reservations(token_jti);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_expires_at ON room_upload_reservations(expires_at);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_chunked_upload ON room_upload_reservations(chunked_upload);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_upload_status ON room_upload_reservations(upload_status);
-- 复合索引：用于查询特定房间的分块上传
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_room_chunked ON room_upload_reservations(room_id, chunked_upload, upload_status);

-- ----------------------------------------------------------------------------
-- room_chunk_uploads 表索引
-- ----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_reservation_id ON room_chunk_uploads(reservation_id);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_upload_status ON room_chunk_uploads(upload_status);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_chunk_index ON room_chunk_uploads(chunk_index);
-- 复合索引：用于查询特定预留的分块状态
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_reservation_status ON room_chunk_uploads(reservation_id, upload_status);

-- ----------------------------------------------------------------------------
-- room_access_logs 表索引
-- ----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_room_access_logs_room_id ON room_access_logs(room_id);
CREATE INDEX IF NOT EXISTS idx_room_access_logs_access_time ON room_access_logs(access_time);
CREATE INDEX IF NOT EXISTS idx_room_access_logs_action ON room_access_logs(action);
-- 复合索引：用于查询特定房间的特定操作日志
CREATE INDEX IF NOT EXISTS idx_room_access_logs_room_action ON room_access_logs(room_id, action, access_time);

-- ============================================================================
-- 触发器定义
-- ============================================================================
-- 说明：触发器用于自动维护数据一致性和完整性
-- 功能：
-- 1. 自动更新 updated_at 时间戳
-- 2. 级联更新相关数据
-- 3. 数据验证和约束
-- ============================================================================

-- ----------------------------------------------------------------------------
-- rooms 表触发器
-- ----------------------------------------------------------------------------
-- 自动更新 updated_at 时间戳
CREATE TRIGGER IF NOT EXISTS trg_rooms_updated_at
AFTER UPDATE ON rooms
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE rooms SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- ----------------------------------------------------------------------------
-- room_contents 表触发器
-- ----------------------------------------------------------------------------
-- 自动更新 updated_at 时间戳
CREATE TRIGGER IF NOT EXISTS trg_room_contents_updated_at
AFTER UPDATE ON room_contents
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE room_contents SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- ----------------------------------------------------------------------------
-- room_upload_reservations 表触发器
-- ----------------------------------------------------------------------------
-- 自动更新 updated_at 时间戳
CREATE TRIGGER IF NOT EXISTS trg_room_upload_reservations_updated_at
AFTER UPDATE ON room_upload_reservations
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE room_upload_reservations SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- ----------------------------------------------------------------------------
-- room_chunk_uploads 表触发器
-- ----------------------------------------------------------------------------
-- 自动更新 updated_at 时间戳
CREATE TRIGGER IF NOT EXISTS trg_room_chunk_uploads_updated_at
AFTER UPDATE ON room_chunk_uploads
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE room_chunk_uploads SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- ============================================================================
-- 视图定义
-- ============================================================================
-- 说明：视图用于简化复杂查询和提供数据抽象
-- 优势：
-- 1. 简化应用层查询逻辑
-- 2. 提供数据安全性（隐藏敏感字段）
-- 3. 提高查询可读性
-- ============================================================================

-- ----------------------------------------------------------------------------
-- v_room_summary - 房间摘要视图
-- ----------------------------------------------------------------------------
-- 描述：提供房间的基本信息和统计数据，隐藏敏感字段（如密码）
-- 用途：用于房间列表展示和概览
-- ----------------------------------------------------------------------------
CREATE VIEW IF NOT EXISTS v_room_summary AS
SELECT
    r.id,
    r.name,
    r.slug,
    CASE WHEN r.password IS NOT NULL THEN TRUE ELSE FALSE END as has_password,
    r.status,
    r.max_size,
    r.current_size,
    CAST(r.current_size AS REAL) / r.max_size * 100 as usage_percentage,
    r.max_times_entered,
    r.current_times_entered,
    r.expire_at,
    r.created_at,
    r.updated_at,
    r.permission,
    COUNT(DISTINCT rc.id) as content_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 0 THEN rc.id END) as text_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 1 THEN rc.id END) as image_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 2 THEN rc.id END) as file_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 3 THEN rc.id END) as url_count
FROM rooms r
LEFT JOIN room_contents rc ON r.id = rc.room_id
GROUP BY r.id;

-- ----------------------------------------------------------------------------
-- v_chunked_upload_status - 分块上传状态视图
-- ----------------------------------------------------------------------------
-- 描述：提供分块上传的详细状态和进度信息
-- 用途：用于监控和管理分块上传任务
-- ----------------------------------------------------------------------------
CREATE VIEW IF NOT EXISTS v_chunked_upload_status AS
SELECT
    rur.id as reservation_id,
    rur.room_id,
    rur.token_jti,
    rur.reserved_size,
    rur.chunked_upload,
    rur.total_chunks,
    rur.uploaded_chunks,
    rur.file_hash,
    rur.chunk_size,
    rur.upload_status,
    rur.expires_at,
    rur.created_at,
    rur.updated_at,
    -- 计算上传进度百分比
    CASE
        WHEN rur.total_chunks IS NULL OR rur.total_chunks = 0 THEN 0.0
        ELSE CAST(rur.uploaded_chunks AS REAL) / rur.total_chunks * 100
    END as upload_progress,
    -- 统计已上传的分块数
    COUNT(rcu.id) as total_uploaded_chunks,
    -- 统计已验证的分块数
    COUNT(CASE WHEN rcu.upload_status = 'uploaded' THEN 1 END) as verified_chunks,
    -- 统计失败的分块数
    COUNT(CASE WHEN rcu.upload_status = 'failed' THEN 1 END) as failed_chunks,
    -- 计算剩余时间（基于平均上传速度，简化估算）
    CASE
        WHEN rur.uploaded_chunks = 0 THEN NULL
        WHEN rur.uploaded_chunks >= rur.total_chunks THEN 0
        ELSE (rur.total_chunks - rur.uploaded_chunks) *
             (CAST((julianday(rur.updated_at) - julianday(rur.created_at)) * 86400 AS INTEGER) / rur.uploaded_chunks)
    END as estimated_remaining_seconds
FROM room_upload_reservations rur
LEFT JOIN room_chunk_uploads rcu ON rur.id = rcu.reservation_id
WHERE rur.chunked_upload = TRUE
GROUP BY rur.id;

-- ----------------------------------------------------------------------------
-- v_active_tokens - 活跃令牌视图
-- ----------------------------------------------------------------------------
-- 描述：提供所有活跃（未过期且未撤销）的访问令牌
-- 用途：用于令牌验证和管理
-- ----------------------------------------------------------------------------
CREATE VIEW IF NOT EXISTS v_active_tokens AS
SELECT
    rt.id,
    rt.room_id,
    rt.jti,
    rt.expires_at,
    rt.created_at,
    r.name as room_name,
    r.slug as room_slug,
    r.status as room_status,
    CAST((julianday(rt.expires_at) - julianday('now')) * 86400 AS INTEGER) as seconds_until_expiry
FROM room_tokens rt
INNER JOIN rooms r ON rt.room_id = r.id
WHERE rt.revoked_at IS NULL
  AND rt.expires_at > CURRENT_TIMESTAMP;

-- ============================================================================
-- 数据完整性和性能优化建议
-- ============================================================================
--
-- 1. 定期清理任务（建议使用 cron job 或定时任务）：
--    - 清理过期的令牌黑名单记录
--    - 清理过期的上传预留记录
--    - 清理过期的房间访问日志（可选，根据审计需求）
--    - 清理过期的房间（根据 expire_at 字段）
--
-- 2. 性能监控：
--    - 监控索引使用情况
--    - 监控慢查询
--    - 监控数据库大小增长
--
-- 3. 备份策略：
--    - 定期备份数据库
--    - 测试恢复流程
--
-- 4. 安全建议：
--    - 定期轮换 JWT 密钥
--    - 监控异常访问模式
--    - 实施速率限制
--
-- ============================================================================
