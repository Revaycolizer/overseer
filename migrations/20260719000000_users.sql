-- 20260719000000_users.sql
-- W1: User authentication & profile storage.

CREATE TABLE IF NOT EXISTS users (
    id           BIGINT PRIMARY KEY,
    username     TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    email        TEXT UNIQUE,
    password_hash TEXT,
    avatar_data  BLOB,
    role         TEXT NOT NULL DEFAULT 'member',
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT,
    is_active    INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS user_sessions (
    id           BIGINT PRIMARY KEY,
    token        TEXT NOT NULL UNIQUE,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at   TEXT NOT NULL,
    last_used_at TEXT,
    user_agent   TEXT,
    ip_address   TEXT
);

CREATE INDEX IF NOT EXISTS idx_user_sessions_user ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_token ON user_sessions(token);
