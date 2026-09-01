CREATE TABLE IF NOT EXISTS file_download_policies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content_id INTEGER NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('off', 'reusable', 'one_time')),
    max_downloads INTEGER,
    download_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (content_id) REFERENCES room_contents (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_file_download_policies_content_id ON file_download_policies(content_id);

CREATE TABLE IF NOT EXISTS file_access_codes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id INTEGER NOT NULL,
    code_hash TEXT NOT NULL,
    is_reusable BOOLEAN NOT NULL DEFAULT 0,
    used_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (policy_id) REFERENCES file_download_policies (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_file_access_codes_policy_hash ON file_access_codes(policy_id, code_hash);
CREATE INDEX IF NOT EXISTS idx_file_access_codes_policy ON file_access_codes(policy_id);
