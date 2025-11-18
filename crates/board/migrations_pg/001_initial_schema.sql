-- PostgreSQL schema for Elizabeth

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
    CHECK (current_size <= max_size),
    CHECK (current_times_entered <= max_times_entered)
);

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
    CHECK (content_type != 0 OR text IS NOT NULL),
    CHECK (content_type != 3 OR url IS NOT NULL),
    CHECK (content_type NOT IN (1, 2) OR (path IS NOT NULL AND file_name IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS room_tokens (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    jti TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS room_refresh_tokens (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS token_blacklist (
    id BIGSERIAL PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    reason TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS room_chunk_uploads (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    upload_id TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    total_chunks BIGINT NOT NULL,
    uploaded_chunks BIGINT NOT NULL DEFAULT 0,
    size BIGINT NOT NULL DEFAULT 0,
    mime_type TEXT,
    status SMALLINT NOT NULL DEFAULT 0 CHECK (status IN (0,1,2)),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS room_upload_reservations (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    reservation_id TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    expected_size BIGINT NOT NULL DEFAULT 0,
    mime_type TEXT,
    expire_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_room_contents_room_id ON room_contents(room_id);
CREATE INDEX IF NOT EXISTS idx_room_tokens_room_id ON room_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_room_id ON room_refresh_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_token_blacklist_expires_at ON token_blacklist(expires_at);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_room_id ON room_chunk_uploads(room_id);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_room_id ON room_upload_reservations(room_id);

-- Updated_at triggers
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_rooms_updated_at BEFORE UPDATE ON rooms
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_room_contents_updated_at BEFORE UPDATE ON room_contents
FOR EACH ROW EXECUTE FUNCTION set_updated_at();
