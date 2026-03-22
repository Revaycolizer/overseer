CREATE TABLE IF NOT EXISTS workflows (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    trigger_type TEXT NOT NULL DEFAULT 'manual',
    trigger_config TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS workflow_steps (
    id BIGINT PRIMARY KEY,
    workflow_id BIGINT NOT NULL,
    step_order INTEGER NOT NULL,
    name TEXT NOT NULL,
    prompt TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workflow_runs (
    id BIGINT PRIMARY KEY,
    workflow_id BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    summary TEXT,
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workflow_step_runs (
    id BIGINT PRIMARY KEY,
    workflow_run_id BIGINT NOT NULL,
    workflow_step_id BIGINT NOT NULL,
    step_order INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    output TEXT,
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_run_id) REFERENCES workflow_runs(id) ON DELETE CASCADE,
    FOREIGN KEY (workflow_step_id) REFERENCES workflow_steps(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_workflow_steps_workflow_order ON workflow_steps(workflow_id, step_order ASC);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_workflow_created ON workflow_runs(workflow_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_workflow_step_runs_run_order ON workflow_step_runs(workflow_run_id, step_order ASC);
