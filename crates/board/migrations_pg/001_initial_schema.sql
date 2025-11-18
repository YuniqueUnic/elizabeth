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
    chunked_upload BOOLEAN DEFAULT FALSE,
    total_chunks BIGINT,
    uploaded_chunks BIGINT DEFAULT 0 CHECK (uploaded_chunks >= 0),
    file_hash TEXT,
    chunk_size BIGINT CHECK (chunk_size IS NULL OR chunk_size > 0),
    upload_status TEXT DEFAULT 'pending' CHECK (upload_status IN ('pending','uploading','completed','failed','expired')),
    CHECK (expires_at > reserved_at),
    CHECK (consumed_at IS NULL OR consumed_at >= reserved_at),
    CHECK (uploaded_chunks IS NULL OR total_chunks IS NULL OR uploaded_chunks <= total_chunks),
    CHECK (chunked_upload = FALSE OR (total_chunks IS NOT NULL AND chunk_size IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS room_chunk_uploads (
    id BIGSERIAL PRIMARY KEY,
    reservation_id BIGINT NOT NULL REFERENCES room_upload_reservations(id) ON DELETE CASCADE,
    chunk_index BIGINT NOT NULL CHECK (chunk_index >= 0),
    chunk_size BIGINT NOT NULL CHECK (chunk_size > 0),
    chunk_hash TEXT,
    upload_status TEXT NOT NULL DEFAULT 'pending' CHECK (upload_status IN ('pending','uploading','uploaded','verified','failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(reservation_id, chunk_index)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_room_contents_room_id ON room_contents(room_id);
CREATE INDEX IF NOT EXISTS idx_room_tokens_room_id ON room_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_room_refresh_tokens_room_id ON room_refresh_tokens(room_id);
CREATE INDEX IF NOT EXISTS idx_token_blacklist_expires_at ON token_blacklist(expires_at);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_room_id ON room_upload_reservations(room_id);
CREATE INDEX IF NOT EXISTS idx_room_upload_reservations_token ON room_upload_reservations(token_jti);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_reservation_id ON room_chunk_uploads(reservation_id);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_chunk_index ON room_chunk_uploads(chunk_index);
CREATE INDEX IF NOT EXISTS idx_room_chunk_uploads_reservation_status ON room_chunk_uploads(reservation_id, upload_status);

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
