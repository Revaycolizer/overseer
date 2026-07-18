-- 20260719000400_collaboration.sql
-- Comments, workflow sharing, and collaboration tables.

CREATE TABLE IF NOT EXISTS workflow_comments (
    id          BIGINT PRIMARY KEY,
    workflow_id BIGINT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    node_id     BIGINT REFERENCES workflow_nodes(id) ON DELETE CASCADE,
    author_id   BIGINT NOT NULL REFERENCES users(id),
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS workflow_shares (
    id          BIGINT PRIMARY KEY,
    workflow_id BIGINT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    shared_with BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission  TEXT NOT NULL DEFAULT 'view',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
