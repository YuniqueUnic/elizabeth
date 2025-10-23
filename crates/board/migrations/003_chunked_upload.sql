-- Elizabeth 项目断点续传功能数据库迁移
-- 创建时间：2025-10-23
-- 版本：3.0.0
-- 描述：添加分块上传支持和断点续传功能

-- ========================================
-- 1. 扩展 room_upload_reservations 表
-- ========================================

-- 添加分块上传相关字段
ALTER TABLE room_upload_reservations ADD COLUMN chunked_upload BOOLEAN DEFAULT FALSE;
ALTER TABLE room_upload_reservations ADD COLUMN total_chunks INTEGER;
ALTER TABLE room_upload_reservations ADD COLUMN uploaded_chunks INTEGER DEFAULT 0;
ALTER TABLE room_upload_reservations ADD COLUMN file_hash TEXT;
ALTER TABLE room_upload_reservations ADD COLUMN chunk_size INTEGER;
ALTER TABLE room_upload_reservations ADD COLUMN upload_status TEXT DEFAULT 'pending';

-- ========================================
-- 2. 房间分块上传表 (room_chunk_uploads)
-- ========================================
CREATE TABLE IF NOT EXISTS room_chunk_uploads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reservation_id INTEGER NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_size INTEGER NOT NULL,
    chunk_hash TEXT,
    upload_status TEXT NOT NULL DEFAULT 'pending',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (reservation_id) REFERENCES room_upload_reservations (id) ON DELETE CASCADE,
    UNIQUE(reservation_id, chunk_index)
);

-- ========================================
-- 3. 索引创建
-- ========================================

-- room_upload_reservations 新索引
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_chunked_upload
    ON room_upload_reservations(chunked_upload);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_upload_status
    ON room_upload_reservations(upload_status);

-- room_chunk_uploads 索引
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_reservation_id
    ON room_chunk_uploads(reservation_id);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_status
    ON room_chunk_uploads(upload_status);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_chunk_index
    ON room_chunk_uploads(chunk_index);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_reservation_status
    ON room_chunk_uploads(reservation_id, upload_status);

-- ========================================
-- 4. 触发器创建
-- ========================================

-- room_chunk_uploads 表 updated_at 触发器
CREATE TRIGGER IF NOT EXISTS trg_room_chunk_uploads_updated_at
AFTER UPDATE ON room_chunk_uploads
FOR EACH ROW
BEGIN
    UPDATE room_chunk_uploads
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;

-- ========================================
-- 5. 视图创建（用于查询上传状态）
-- ========================================

-- 分块上传状态视图
CREATE VIEW IF NOT EXISTS v_chunked_upload_status AS
SELECT
    rur.id as reservation_id,
    rur.room_id,
    rur.chunked_upload,
    rur.total_chunks,
    rur.uploaded_chunks,
    rur.file_hash,
    rur.chunk_size,
    rur.upload_status,
    rur.expires_at,
    CASE
        WHEN rur.total_chunks IS NULL THEN 0.0
        WHEN rur.total_chunks = 0 THEN 0.0
        ELSE CAST(rur.uploaded_chunks AS REAL) / rur.total_chunks * 100
    END as upload_progress,
    COUNT(rcu.id) as total_uploaded_chunks,
    COUNT(CASE WHEN rcu.upload_status = 'uploaded' THEN 1 END) as verified_chunks
FROM room_upload_reservations rur
LEFT JOIN room_chunk_uploads rcu ON rur.id = rcu.reservation_id
WHERE rur.chunked_upload = TRUE
GROUP BY rur.id;

-- ========================================
-- 6. 数据完整性检查
-- ========================================

-- 确保现有数据的兼容性
UPDATE room_upload_reservations
SET upload_status = CASE
    WHEN consumed_at IS NOT NULL THEN 'completed'
    WHEN expires_at < CURRENT_TIMESTAMP THEN 'expired'
    ELSE 'pending'
END
WHERE upload_status IS NULL OR upload_status = 'pending';

-- ========================================
-- 7. 性能优化建议注释
-- ========================================

-- 注意：对于大量分块上传的场景，建议考虑以下优化：
-- 1. 定期清理过期的分块数据
-- 2. 对大表进行分区（如果数据量很大）
-- 3. 考虑使用专门的文件存储系统管理分块文件
-- 4. 添加监控指标跟踪分块上传性能
