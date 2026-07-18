-- 20260718020000_compaction_context_ratio.sql
-- V4.17.16: compaction trigger as a fraction of the model's num_ctx.
-- The old compaction_token_threshold setting is superseded by
-- compaction_context_ratio (default 0.75 = compact when estimated
-- tokens exceed 75% of the active model's context window). The
-- threshold is now dynamic: the compactor queries Ollama's /api/show
-- for the resolved model's num_ctx and multiplies by this ratio.

INSERT OR IGNORE INTO settings (key, value) VALUES
    ('compaction_context_ratio', '0.75');
