CREATE TABLE learning_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL CHECK(kind IN ('preference','project','outcome')),
    scope TEXT NOT NULL,
    memory_key TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(kind, scope, memory_key)
);
CREATE INDEX learning_memories_scope ON learning_memories(scope, kind);
