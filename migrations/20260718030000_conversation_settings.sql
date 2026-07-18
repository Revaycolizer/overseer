-- 20260718030000_conversation_settings.sql
-- Per-conversation overrides for model, parameters, etc.
-- Key-value store scoped to a conversation. When present, the
-- orchestrator reads these values instead of the global settings.

CREATE TABLE IF NOT EXISTS conversation_settings (
    conversation_id BIGINT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (conversation_id, key),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);
