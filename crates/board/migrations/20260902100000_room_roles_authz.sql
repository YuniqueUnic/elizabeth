-- ============================================================================
-- 房间角色与权限系统重构（docs/PERMISSIONS_REDESIGN.md §6）
-- room_roles 授权矩阵 + token 绑定角色 + 内容创建者归属 + 删除旧 permission 位
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. room_roles：每房间角色矩阵（授权唯一真相；复合主键即 (room_id, role_key)）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS room_roles (
    room_id      INTEGER NOT NULL REFERENCES rooms (id) ON DELETE CASCADE,
    role_key     TEXT    NOT NULL,
    display_name TEXT    NOT NULL,
    capabilities TEXT    NOT NULL DEFAULT '[]',
    is_system    INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (room_id, role_key)
);

-- ----------------------------------------------------------------------------
-- 2. rooms：默认加入角色 + 角色矩阵版本号（RoleTable 缓存失效依据）
-- ----------------------------------------------------------------------------
ALTER TABLE rooms ADD COLUMN default_role_key TEXT NOT NULL DEFAULT 'editor';
ALTER TABLE rooms ADD COLUMN roles_version INTEGER NOT NULL DEFAULT 1;

-- ----------------------------------------------------------------------------
-- 3. token 绑定角色；内容记录创建者（own 作用域判定依据）
-- ----------------------------------------------------------------------------
ALTER TABLE room_tokens ADD COLUMN role_key TEXT;
ALTER TABLE room_contents ADD COLUMN created_by_jti TEXT;
CREATE INDEX IF NOT EXISTS idx_room_contents_owner
    ON room_contents (room_id, created_by_jti);

-- ----------------------------------------------------------------------------
-- 4. 存量映射：旧 permission 位 → 角色能力集（保持既有会话的有效权力不缩水）
--    bits&8=DELETE → editor 升为房间管理；bits&4=SHARE → editor/reader 并入 room.share
--    紧凑格式：无后缀 = Any，":own" = 仅自己
-- ----------------------------------------------------------------------------
INSERT INTO room_roles (room_id, role_key, display_name, capabilities, is_system)
SELECT r.id, 'manager', 'Manager',
       '["room.share","room.settings.update","room.roles.manage","room.delete","msg.read","msg.send","msg.copy","msg.edit","msg.delete","file.list","file.preview","file.download","file.upload","file.delete","file.policy.manage"]',
       1
FROM rooms r;

INSERT INTO room_roles (room_id, role_key, display_name, capabilities, is_system)
SELECT r.id, 'editor', 'Editor',
       CASE
           WHEN (r.permission & 8) != 0 THEN '["room.share","room.settings.update","room.roles.manage","room.delete","msg.read","msg.send","msg.copy","msg.edit","msg.delete","file.list","file.preview","file.download","file.upload","file.delete","file.policy.manage"]'
           WHEN (r.permission & 4) != 0 THEN '["room.share","msg.read","msg.send","msg.copy","msg.edit","msg.delete:own","file.list","file.preview","file.download","file.upload","file.delete:own"]'
           ELSE '["msg.read","msg.send","msg.copy","msg.edit","msg.delete:own","file.list","file.preview","file.download","file.upload","file.delete:own"]'
       END,
       1
FROM rooms r;

INSERT INTO room_roles (room_id, role_key, display_name, capabilities, is_system)
SELECT r.id, 'reader', 'Reader',
       CASE
           WHEN (r.permission & 4) != 0 THEN '["room.share","msg.read","msg.copy","file.list","file.preview","file.download"]'
           ELSE '["msg.read","msg.copy","file.list","file.preview","file.download"]'
       END,
       1
FROM rooms r;

-- ----------------------------------------------------------------------------
-- 5. 存量 token → 角色映射（&8→manager / &2→editor / 其余→reader）
-- ----------------------------------------------------------------------------
UPDATE room_tokens
SET role_key = (
    SELECT CASE
               WHEN (r.permission & 8) != 0 THEN 'manager'
               WHEN (r.permission & 2) != 0 THEN 'editor'
               ELSE 'reader'
           END
    FROM rooms r
    WHERE r.id = room_tokens.room_id
);

-- ----------------------------------------------------------------------------
-- 6. 视图不再暴露 permission：先删视图 → 移除旧列 → 重建视图
-- ----------------------------------------------------------------------------
DROP VIEW IF EXISTS v_room_summary;
ALTER TABLE rooms DROP COLUMN permission;

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
    COUNT(DISTINCT rc.id) as content_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 0 THEN rc.id END) as text_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 1 THEN rc.id END) as image_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 2 THEN rc.id END) as file_count,
    COUNT(DISTINCT CASE WHEN rc.content_type = 3 THEN rc.id END) as url_count
FROM rooms r
LEFT JOIN room_contents rc ON r.id = rc.room_id
GROUP BY r.id;
