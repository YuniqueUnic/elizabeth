CREATE TABLE room_identity_codes (
    id BIGSERIAL PRIMARY KEY,
    room_id BIGINT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    role_key TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    created_by_jti TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP::TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP::TEXT,
    UNIQUE (room_id, code_hash)
);

CREATE INDEX idx_room_identity_codes_room_active
    ON room_identity_codes(room_id, revoked_at, expires_at);

ALTER TABLE room_tokens
    ADD COLUMN identity_code_id BIGINT REFERENCES room_identity_codes(id) ON DELETE SET NULL;

CREATE INDEX idx_room_tokens_identity_code_id
    ON room_tokens(identity_code_id);
