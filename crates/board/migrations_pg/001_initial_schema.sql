-- ============================================================================
-- Elizabeth 项目 PostgreSQL 数据库架构
-- ============================================================================
-- 项目：Elizabeth - 文件分享与协作平台
-- 版本：1.0.0
-- 数据库：PostgreSQL 15+
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
CREATE TABLE IF NOT EXISTS rooms (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    password TEXT,
    status SMALLINT NOT NULL DEFAULT 0 CHECK (status IN (0, 1, 2)),
    max_size BIGINT NOT NULL DEFAULT 10485760 CHECK (max_size > 0),
    current_size BIGINT NOT NULL DEFAULT 0 CHECK (current_size >= 0),
    max_times_entered BIGINT NOT NULL DEFAULT 100 CHECK (max_times_entered > 0),
    current_times_entered BIGINT NOT NULL DEFAULT 0 CHECK (current_times_entered >= 0),
    expire_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    permission BIGINT NOT NULL DEFAULT 1 CHECK (permission >= 0),

    -- 约束：当前容量不能超过最大容量
    CHECK (current_size <= max_size),
    -- 约束：当前访问次数不能超过最大访问次数
    CHECK (current_times_entered <= max_times_entered)
);

-- ----------------------------------------------------------------------------
-- 2. 房间内容表 (room_contents)
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_contents (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    content_type SMALLINT NOT NULL CHECK (content_type IN (0, 1, 2, 3)),
    text TEXT,
    url TEXT,
    path TEXT,
    file_name TEXT,
    size BIGINT DEFAULT 0 CHECK (size >= 0),
    mime_type TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

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
CREATE TABLE IF NOT EXISTS room_tokens (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    jti TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 约束：过期时间必须在创建时间之后
    CHECK (expires_at > created_at),
    -- 约束：撤销时间必须在创建时间之后
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

-- ----------------------------------------------------------------------------
-- 4. 房间刷新令牌表 (room_refresh_tokens)
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_refresh_tokens (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    access_token_jti TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,

    -- 约束：过期时间必须在创建时间之后
    CHECK (expires_at > created_at),
    -- 约束：最后使用时间必须在创建时间之后
    CHECK (last_used_at IS NULL OR last_used_at >= created_at)
);

-- ----------------------------------------------------------------------------
-- 5. 令牌黑名单表 (token_blacklist)
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS token_blacklist (
    id BIGSERIAL PRIMARY KEY,
    jti TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 约束：过期时间必须在创建时间之后
    CHECK (expires_at > created_at)
);

-- ----------------------------------------------------------------------------
-- 6. 上传预留表 (room_upload_reservations)
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_upload_reservations (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    token_jti TEXT NOT NULL,
    file_manifest TEXT NOT NULL,
    reserved_size BIGINT NOT NULL CHECK (reserved_size > 0),
    reserved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 分块上传相关字段
    chunked_upload BOOLEAN DEFAULT FALSE,
    total_chunks BIGINT,
    uploaded_chunks BIGINT DEFAULT 0 CHECK (uploaded_chunks >= 0),
    file_hash TEXT,
    chunk_size BIGINT CHECK (chunk_size IS NULL OR chunk_size > 0),
    upload_status TEXT DEFAULT 'pending' CHECK (upload_status IN ('pending', 'uploading', 'completed', 'failed', 'expired')),

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
CREATE TABLE IF NOT EXISTS room_chunk_uploads (
    id BIGSERIAL PRIMARY KEY,
    reservation_id BIGINT NOT NULL REFERENCES room_upload_reservations(id) ON DELETE CASCADE,
    chunk_index BIGINT NOT NULL CHECK (chunk_index >= 0),
    chunk_size BIGINT NOT NULL CHECK (chunk_size > 0),
    chunk_hash TEXT,
    upload_status TEXT NOT NULL DEFAULT 'pending' CHECK (upload_status IN ('pending', 'uploading', 'uploaded', 'verified', 'failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 约束：同一预留的分块索引必须唯一
    UNIQUE(reservation_id, chunk_index)
);

-- ----------------------------------------------------------------------------
-- 8. 房间访问日志表 (room_access_logs)
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_access_logs (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    ip_address TEXT,
    user_agent TEXT,
    access_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    action SMALLINT NOT NULL,
    details TEXT
);

-- ============================================================================
-- 索引定义
-- ============================================================================
-- 说明：索引用于优化查询性能，根据实际查询模式设计
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
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 自动更新 updated_at 时间戳的函数
-- ----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ----------------------------------------------------------------------------
-- rooms 表触发器
-- ----------------------------------------------------------------------------
CREATE TRIGGER trg_rooms_updated_at
BEFORE UPDATE ON rooms
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- ----------------------------------------------------------------------------
-- room_contents 表触发器
-- ----------------------------------------------------------------------------
CREATE TRIGGER trg_room_contents_updated_at
BEFORE UPDATE ON room_contents
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- ----------------------------------------------------------------------------
-- room_upload_reservations 表触发器
-- ----------------------------------------------------------------------------
CREATE TRIGGER trg_room_upload_reservations_updated_at
BEFORE UPDATE ON room_upload_reservations
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- ----------------------------------------------------------------------------
-- room_chunk_uploads 表触发器
-- ----------------------------------------------------------------------------
CREATE TRIGGER trg_room_chunk_uploads_updated_at
BEFORE UPDATE ON room_chunk_uploads
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- ============================================================================
-- 视图定义
-- ============================================================================
-- 说明：视图用于简化复杂查询和提供数据抽象
-- ============================================================================

-- ----------------------------------------------------------------------------
-- v_room_summary - 房间摘要视图
-- ----------------------------------------------------------------------------
CREATE OR REPLACE VIEW v_room_summary AS
SELECT
    r.id,
    r.name,
    r.slug,
    CASE WHEN r.password IS NOT NULL THEN TRUE ELSE FALSE END as has_password,
    r.status,
    r.max_size,
    r.current_size,
    CAST(r.current_size AS NUMERIC) / r.max_size * 100 as usage_percentage,
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
CREATE OR REPLACE VIEW v_chunked_upload_status AS
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
        ELSE CAST(rur.uploaded_chunks AS NUMERIC) / rur.total_chunks * 100
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
             EXTRACT(EPOCH FROM (rur.updated_at - rur.created_at)) / rur.uploaded_chunks
    END as estimated_remaining_seconds
FROM room_upload_reservations rur
LEFT JOIN room_chunk_uploads rcu ON rur.id = rcu.reservation_id
WHERE rur.chunked_upload = TRUE
GROUP BY rur.id;

-- ----------------------------------------------------------------------------
-- v_active_tokens - 活跃令牌视图
-- ----------------------------------------------------------------------------
CREATE OR REPLACE VIEW v_active_tokens AS
SELECT
    rt.id,
    rt.room_id,
    rt.jti,
    rt.expires_at,
    rt.created_at,
    r.name as room_name,
    r.slug as room_slug,
    r.status as room_status,
    EXTRACT(EPOCH FROM (rt.expires_at - NOW())) as seconds_until_expiry
FROM room_tokens rt
INNER JOIN rooms r ON rt.room_id = r.id
WHERE rt.revoked_at IS NULL
  AND rt.expires_at > NOW();

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
