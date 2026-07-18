-- 20260719000300_workflow_versioning.sql
-- Workflow versions, publish state, and templates.

ALTER TABLE workflows ADD COLUMN status TEXT NOT NULL DEFAULT 'draft';
ALTER TABLE workflows ADD COLUMN published_at TEXT;

CREATE TABLE IF NOT EXISTS workflow_versions (
    id          BIGINT PRIMARY KEY,
    workflow_id BIGINT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    version     INTEGER NOT NULL,
    graph_json  TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    created_by  BIGINT REFERENCES users(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_workflow_versions_wv
    ON workflow_versions(workflow_id, version);

CREATE TABLE IF NOT EXISTS workflow_templates (
    id          BIGINT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    category    TEXT NOT NULL DEFAULT 'general',
    graph_json  TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
