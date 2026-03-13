-- Create versions table for history
CREATE TABLE IF NOT EXISTS versions (
    id BIGINT PRIMARY KEY,
    entity_type TEXT NOT NULL, -- 'agent' or 'tool'
    entity_id BIGINT NOT NULL,
    version_tag TEXT NOT NULL,
    content TEXT NOT NULL, -- JSON for agents, source code for tools
    changelog TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Ensure tools table has content column if missing (it's 'code' in the existing schema)
-- Our existing 'tools' table has 'code' and 'script_path'.
-- We will keep them for the "active" state, but the 'versions' table will hold the history.

-- Index for faster lookup of version history
CREATE INDEX IF NOT EXISTS idx_versions_entity ON versions(entity_type, entity_id);
