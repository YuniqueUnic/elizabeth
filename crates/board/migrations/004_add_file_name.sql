-- 添加 file_name 字段到 room_contents 表
-- 用于存储原始文件名，与磁盘上的 UUID 文件名分离
-- 创建时间：2025-11-01

-- 添加 file_name 字段
ALTER TABLE room_contents ADD COLUMN file_name TEXT;

-- 为现有记录从 path 字段提取文件名（如果存在）
-- 这个更新会尝试从路径中提取文件名，但对于新上传的文件，
-- 我们将在上传时直接设置 file_name
UPDATE room_contents
SET file_name = (
    SELECT
        CASE
            WHEN path IS NOT NULL THEN
                -- 从路径中提取文件名（去掉目录部分）
                substr(path, instr(path, '/') + 1)
            ELSE NULL
        END
)
WHERE path IS NOT NULL AND file_name IS NULL;
