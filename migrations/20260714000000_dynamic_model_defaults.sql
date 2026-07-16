-- 20260714000000_dynamic_model_defaults.sql
-- Move model-related setting seeds out of the backend (src/db/seed_core.rs)
-- into a migration so all DB bootstrap lives in /migrations. The
-- planner/chat models must be inserted as "use-first-available"
-- placeholders at migration time and resolved to the actual first
-- model in Ollama at runtime, because the migration runs before
-- Ollama is queried and most installs don't ship a hardcoded
-- 'qwen2.5:7b-instruct'.
--
-- Values use the sentinel string '__use_first_available__' which
-- resolve_model_name already understands: when the configured value
-- equals this sentinel (or any other unrecognised name), the resolver
-- falls back to the first model in Ollama's /api/tags list, with the
-- existing hard_default as the last-resort name.

INSERT OR IGNORE INTO settings (key, value) VALUES
    ('title_generator_model',   '__use_first_available__'),
    ('default_chat_model',      '__use_first_available__'),
    ('planner_model',           '__use_first_available__'),
    ('planner_model_fallbacks',  '[]'),
    ('vision_model',            '__use_first_available__'),
    ('router_model',            '__use_first_available__');
