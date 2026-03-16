-- ═══════════════════════════════════════════════════════════════
-- Snowflake ID Migration (Safe Transformation)
-- ═══════════════════════════════════════════════════════════════

-- 1. Standardize settings table
CREATE TABLE settings_new (
    id BIGINT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert existing settings, id will be populated later by Rust or default to NULL/0 for now
INSERT INTO settings_new (key, value, updated_at)
SELECT key, value, updated_at FROM settings;

DROP TABLE settings;
ALTER TABLE settings_new RENAME TO settings;

-- 2. Standardize agent_tools table (many-to-many with primary ID)
CREATE TABLE agent_tools_new (
    id BIGINT PRIMARY KEY,
    agent_id BIGINT NOT NULL,
    tool_id BIGINT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    FOREIGN KEY(tool_id) REFERENCES tools(id) ON DELETE CASCADE,
    UNIQUE(agent_id, tool_id)
);

-- Preserve existing relationships
-- Note: existing agent_id/tool_id are kept. Snowflake id for this table will be generated in seed/runtime.
INSERT INTO agent_tools_new (agent_id, tool_id)
SELECT agent_id, tool_id FROM agent_tools;

DROP TABLE agent_tools;
ALTER TABLE agent_tools_new RENAME TO agent_tools;

-- 3. Standardize attachments table if it exists as INTEGER
CREATE TABLE attachments_new (
    id BIGINT PRIMARY KEY NOT NULL,
    message_id BIGINT NOT NULL,
    filename TEXT NOT NULL,
    file_path TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size BIGINT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(message_id) REFERENCES messages(id) ON DELETE CASCADE
);

INSERT INTO attachments_new (id, message_id, filename, file_path, content_type, size, created_at)
SELECT id, message_id, filename, file_path, content_type, size, created_at FROM attachments;

DROP TABLE attachments;
ALTER TABLE attachments_new RENAME TO attachments;
