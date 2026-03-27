CREATE TABLE IF NOT EXISTS artifacts (
    id BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL,
    execution_run_id BIGINT,
    domain TEXT,
    step_text TEXT,
    kind TEXT NOT NULL,
    label TEXT NOT NULL,
    content TEXT,
    file_path TEXT,
    content_type TEXT,
    metadata_json TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (execution_run_id) REFERENCES execution_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_artifacts_conversation_created_at
    ON artifacts(conversation_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_artifacts_run_created_at
    ON artifacts(execution_run_id, created_at ASC);
