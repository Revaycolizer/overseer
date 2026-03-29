CREATE TABLE IF NOT EXISTS work_item_events (
    id BIGINT PRIMARY KEY,
    work_item_id BIGINT NOT NULL,
    execution_run_id BIGINT,
    event_type TEXT NOT NULL,
    status TEXT,
    summary TEXT,
    metadata_json TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (work_item_id) REFERENCES work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (execution_run_id) REFERENCES execution_runs(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_work_item_events_work_item_created_at
    ON work_item_events(work_item_id, created_at DESC);
