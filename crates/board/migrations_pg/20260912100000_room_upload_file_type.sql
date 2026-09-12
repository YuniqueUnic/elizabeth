-- 房间级上传文件类型策略：any 不限制；allow 仅允许列表内扩展名；deny 拒绝列表内扩展名。
-- 扩展名统一小写、不含点，以 JSON 数组字符串存储。
ALTER TABLE rooms ADD COLUMN upload_file_type_mode TEXT NOT NULL DEFAULT 'any'
    CHECK (upload_file_type_mode IN ('any', 'allow', 'deny'));
ALTER TABLE rooms ADD COLUMN upload_file_type_extensions TEXT NOT NULL DEFAULT '[]';
