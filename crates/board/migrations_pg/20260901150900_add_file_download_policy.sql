CREATE TABLE IF NOT EXISTS file_download_policies (
    id BIGSERIAL PRIMARY KEY,
    content_id BIGINT NOT NULL REFERENCES room_contents(id) ON DELETE CASCADE,
    mode TEXT NOT NULL CHECK (mode IN ('off', 'reusable', 'one_time')),
    max_downloads INTEGER,
    download_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'),
    updated_at TEXT NOT NULL DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_file_download_policies_content_id ON file_download_policies(content_id);

CREATE TABLE IF NOT EXISTS file_access_codes (
    id BIGSERIAL PRIMARY KEY,
    policy_id BIGINT NOT NULL REFERENCES file_download_policies(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    is_reusable BOOLEAN NOT NULL DEFAULT FALSE,
    used_at TEXT,
    created_at TEXT NOT NULL DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_file_access_codes_policy_hash ON file_access_codes(policy_id, code_hash);
CREATE INDEX IF NOT EXISTS idx_file_access_codes_policy ON file_access_codes(policy_id);
