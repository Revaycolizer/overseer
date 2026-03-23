CREATE TABLE IF NOT EXISTS tool_test_cases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    input TEXT NOT NULL DEFAULT '',
    expected_substring TEXT,
    last_status TEXT,
    last_output TEXT,
    last_run_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(tool_id) REFERENCES tools(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tool_test_cases_tool_id
    ON tool_test_cases(tool_id);
