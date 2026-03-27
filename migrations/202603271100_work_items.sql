CREATE TABLE IF NOT EXISTS work_items (
    id BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL,
    title TEXT NOT NULL,
    brief TEXT,
    status TEXT NOT NULL DEFAULT 'queued',
    surface_kind TEXT NOT NULL DEFAULT 'summary',
    assigned_domain TEXT,
    prompt TEXT NOT NULL,
    source_artifact_ids_json TEXT,
    metadata_json TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_work_items_conversation_updated_at
    ON work_items(conversation_id, updated_at DESC, created_at DESC);
