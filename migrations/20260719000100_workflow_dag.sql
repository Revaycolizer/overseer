-- 20260719000100_workflow_dag.sql
-- W2: DAG-based workflow nodes and edges, replacing linear workflow_steps.

CREATE TABLE IF NOT EXISTS workflow_nodes (
    id            BIGINT PRIMARY KEY,
    workflow_id   BIGINT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    node_type     TEXT NOT NULL,
    label         TEXT NOT NULL DEFAULT 'Node',
    position_x    REAL NOT NULL DEFAULT 0,
    position_y    REAL NOT NULL DEFAULT 0,
    config_json   TEXT NOT NULL DEFAULT '{}',
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS workflow_edges (
    id          BIGINT PRIMARY KEY,
    workflow_id BIGINT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    source_node BIGINT NOT NULL REFERENCES workflow_nodes(id) ON DELETE CASCADE,
    target_node BIGINT NOT NULL REFERENCES workflow_nodes(id) ON DELETE CASCADE,
    source_port TEXT NOT NULL DEFAULT 'output',
    target_port TEXT NOT NULL DEFAULT 'input'
);

ALTER TABLE workflows ADD COLUMN owner_id BIGINT REFERENCES users(id);

CREATE TABLE IF NOT EXISTS workflow_node_runs (
    id              BIGINT PRIMARY KEY,
    workflow_run_id BIGINT NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    node_id         BIGINT NOT NULL REFERENCES workflow_nodes(id) ON DELETE SET NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    input_json      TEXT,
    output_json     TEXT,
    error           TEXT,
    started_at      TEXT,
    completed_at    TEXT
);
