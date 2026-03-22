-- Runtime portability, observability, and execution safety defaults

ALTER TABLE tools ADD COLUMN safety_level TEXT NOT NULL DEFAULT 'standard';

CREATE TABLE IF NOT EXISTS execution_runs (
    id BIGINT PRIMARY KEY,
    conversation_id BIGINT,
    user_query TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    final_response TEXT,
    error_message TEXT,
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS execution_events (
    id BIGINT PRIMARY KEY,
    run_id BIGINT NOT NULL,
    sequence INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'info',
    domain TEXT,
    step_text TEXT,
    tool_name TEXT,
    model TEXT,
    message TEXT,
    details TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (run_id) REFERENCES execution_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_execution_runs_conversation ON execution_runs(conversation_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_execution_events_run_sequence ON execution_events(run_id, sequence ASC);

INSERT OR IGNORE INTO settings (id, key, value) VALUES (1721827364582, 'tool_timeout_ms', '45000');
INSERT OR IGNORE INTO settings (id, key, value) VALUES (1721827364583, 'model_timeout_ms', '90000');
