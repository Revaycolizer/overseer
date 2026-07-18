-- 20260719000200_credentials.sql
-- Encrypted credentials store for workflow nodes (API keys, tokens, etc.)

CREATE TABLE IF NOT EXISTS credentials (
    id          BIGINT PRIMARY KEY,
    name        TEXT NOT NULL,
    cred_type   TEXT NOT NULL DEFAULT 'api_key',
    encrypted_value TEXT NOT NULL,
    owner_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
