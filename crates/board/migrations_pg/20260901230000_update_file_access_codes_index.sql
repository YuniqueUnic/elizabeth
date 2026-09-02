DROP INDEX IF EXISTS idx_file_access_codes_hash;
CREATE UNIQUE INDEX IF NOT EXISTS idx_file_access_codes_policy_hash ON file_access_codes(policy_id, code_hash);
