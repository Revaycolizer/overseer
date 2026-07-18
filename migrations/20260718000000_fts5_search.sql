-- 20260718000000_fts5_search.sql
-- V4.17.11: FTS5 full-text search index on messages.content.
-- The Dioxus sidebar and TUI /search command already call
-- search_messages(); this migration makes those queries
-- use a proper inverted index instead of LIKE '%query%'.

CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    content=messages,
    content_rowid=id
);

CREATE TRIGGER IF NOT EXISTS messages_fts_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.id, old.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_au AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.id, old.content);
    INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
END;

-- Backfill any existing messages that predate this migration.
INSERT INTO messages_fts(rowid, content) SELECT id, content FROM messages;
