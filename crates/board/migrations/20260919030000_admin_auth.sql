-- 平台管理员账号：bootstrap 由环境变量创建一次，密码修改经 API 持久化（Argon2）。
CREATE TABLE admin_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 平台管理 API key：明文仅在创建响应中出现一次，库中仅存 SHA-256 与可定位前缀。
CREATE TABLE admin_api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    prefix TEXT NOT NULL UNIQUE,
    key_hash TEXT NOT NULL,
    expires_at DATETIME,
    revoked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME,
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX idx_admin_api_keys_active ON admin_api_keys(revoked_at);
