CREATE TABLE room_identity_codes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    code_hash TEXT NOT NULL,
    role_key TEXT NOT NULL,
    expires_at DATETIME NOT NULL,
    revoked_at DATETIME,
    created_by_jti TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE,
    UNIQUE (room_id, code_hash),
    CHECK (expires_at > created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX idx_room_identity_codes_room_active
    ON room_identity_codes(room_id, revoked_at, expires_at);

ALTER TABLE room_tokens
    ADD COLUMN identity_code_id INTEGER REFERENCES room_identity_codes(id) ON DELETE SET NULL;

CREATE INDEX idx_room_tokens_identity_code_id
    ON room_tokens(identity_code_id);
