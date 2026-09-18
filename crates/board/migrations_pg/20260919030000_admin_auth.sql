-- 平台管理员账号：bootstrap 由环境变量创建一次，密码修改经 API 持久化（Argon2）。
CREATE TABLE admin_accounts (
    id BIGSERIAL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP::TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP::TEXT
);

-- 平台管理 API key：明文仅在创建响应中出现一次，库中仅存 SHA-256 与可定位前缀。
CREATE TABLE admin_api_keys (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    prefix TEXT NOT NULL UNIQUE,
    key_hash TEXT NOT NULL,
    expires_at TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP::TEXT,
    last_used_at TEXT,
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX idx_admin_api_keys_active ON admin_api_keys(revoked_at);
