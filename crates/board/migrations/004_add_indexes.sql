-- 为房间表添加索引
CREATE INDEX IF NOT EXISTS idx_rooms_name ON rooms(name);
CREATE INDEX IF NOT EXISTS idx_rooms_status ON rooms(status);
CREATE INDEX IF NOT EXISTS idx_rooms_expire_at ON rooms(expire_at);
CREATE INDEX IF NOT EXISTS idx_rooms_created_at ON rooms(created_at);

-- 为房间内容表添加索引
CREATE INDEX IF NOT EXISTS idx_room_contents_room_id ON room_contents(room_id);
CREATE INDEX IF NOT EXISTS idx_room_contents_content_type ON room_contents(content_type);
CREATE INDEX IF NOT EXISTS idx_room_contents_created_at ON room_contents(created_at);

-- 为房间访问日志表添加索引
CREATE INDEX IF NOT EXISTS idx_room_access_logs_room_id ON room_access_logs(room_id);
CREATE INDEX IF NOT EXISTS idx_room_access_logs_access_time ON room_access_logs(access_time);
CREATE INDEX IF NOT EXISTS idx_room_access_logs_action ON room_access_logs(action);
