-- 20260718040000_browser_sessions.sql
-- Browser automation pool: tracks Obscura processes, their state,
-- and per-session recording metadata.

CREATE TABLE IF NOT EXISTS browser_sessions (
    id                   BIGINT PRIMARY KEY,
    port                 INTEGER NOT NULL UNIQUE,
    profile_path         TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'active',
    created_at           TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at         TEXT NOT NULL DEFAULT (datetime('now')),
    ttl_seconds          INTEGER NOT NULL DEFAULT 3600,
    conversation_id      INTEGER,
    recording_state      TEXT DEFAULT 'off',
    recording_path       TEXT,
    recording_size_bytes INTEGER,
    recording_duration_ms INTEGER,
    recording_started_at TEXT,
    recording_ended_at   TEXT
);

INSERT OR IGNORE INTO settings (key, value) VALUES
    ('browser_warn_ram_pct',      '70'),
    ('browser_critical_ram_pct',  '85'),
    ('browser_block_ram_pct',     '95'),
    ('browser_max_sessions',      '8'),
    ('browser_per_session_mb',    '90'),
    ('browser_auto_suspend_idle', 'true');
