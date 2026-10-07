-- Keep existing IDs and scopes while allowing inspectable, unverified tool notes.
CREATE TABLE learning_memories_next (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL CHECK(kind IN ('preference','project','outcome','note')),
    scope TEXT NOT NULL,
    memory_key TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(kind, scope, memory_key)
);
INSERT INTO learning_memories_next SELECT * FROM learning_memories;
INSERT INTO sqlite_sequence(name, seq) SELECT 'learning_memories_next', seq FROM sqlite_sequence WHERE name='learning_memories' AND NOT EXISTS (SELECT 1 FROM sqlite_sequence WHERE name='learning_memories_next');
UPDATE sqlite_sequence SET seq = MAX(seq, COALESCE((SELECT seq FROM sqlite_sequence WHERE name='learning_memories'), 0)) WHERE name='learning_memories_next';
DROP TABLE learning_memories;
ALTER TABLE learning_memories_next RENAME TO learning_memories;
CREATE INDEX learning_memories_scope ON learning_memories(scope, kind);
