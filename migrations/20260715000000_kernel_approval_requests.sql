-- Overseer Kernel v0.1: persistent approval ledger.
--
-- Authoritative store for human-in-the-loop approval decisions on agent actions.
-- Replaces the previous practice of tracking approval state in generic `settings`
-- rows. The table shape below is identical to `overseer_kernel::APPROVAL_SCHEMA`
-- (the kernel crate applies that constant to its in-memory test database).

CREATE TABLE IF NOT EXISTS approval_requests (
    id             TEXT PRIMARY KEY,
    run_id         BIGINT NOT NULL,
    action_id      TEXT NOT NULL,
    action_kind    TEXT NOT NULL,
    tool_name      TEXT NOT NULL,
    action_payload TEXT NOT NULL,
    payload_hash   TEXT NOT NULL,
    summary        TEXT,
    status         TEXT NOT NULL DEFAULT 'pending',
    reason         TEXT,
    required_role  TEXT,
    policy_version TEXT,
    policy_snapshot TEXT,
    requested_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved_at    DATETIME,
    resolver       TEXT,
    resolution     TEXT
);

CREATE INDEX IF NOT EXISTS idx_approval_requests_run_status
    ON approval_requests(run_id, status);

-- Restart recovery: any approval left `pending` by a previous process can never
-- be resolved (its in-memory async waiter did not survive the restart), so
-- cancel stale pending rows when this migration is (re)applied on a fresh boot.
UPDATE approval_requests
   SET status = 'cancelled',
       resolver = 'system:restart',
       resolution = 'cancelled on application restart',
       resolved_at = CURRENT_TIMESTAMP
 WHERE status = 'pending';

-- Kernel policy defaults (mirrors the existing computer-supervision settings so
-- the deterministic policy has explicit inputs).
INSERT OR IGNORE INTO settings (id, key, value) VALUES (1752566400001, 'kernel_supervision_mode', 'approval_required');
INSERT OR IGNORE INTO settings (id, key, value) VALUES (1752566400002, 'kernel_max_actions', '25');
INSERT OR IGNORE INTO settings (id, key, value) VALUES (1752566400003, 'kernel_session_timeout_ms', '600000');
INSERT OR IGNORE INTO settings (id, key, value) VALUES (1752566400004, 'kernel_approval_wait_ms', '300000');
