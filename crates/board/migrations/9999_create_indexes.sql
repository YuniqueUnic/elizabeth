-- 9999_create_indexes.sql
-- 为所有表创建索引

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
