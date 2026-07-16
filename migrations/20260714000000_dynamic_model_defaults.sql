-- 20260714000000_dynamic_model_defaults.sql
-- V4.17 dynamic model defaults. Move model-related setting seeds
-- out of the backend (src/db/seed_core.rs) into a migration so all
-- DB bootstrap lives in /migrations. The 5 model rows below are
-- the only global fallbacks that the orchestrator subsystems need:
--
--   * planner_model / planner_model_fallbacks
--       The master planner. Per-agent / per-supervisor models live
--       in the `agents` / `supervisors` tables; this is just the
--       global fallback when the per-agent model isn't installed.
--   * title_generator_model
--       The lighter model used for naming new conversations
--       (`generate_title` in agent_system.orchestrate).
--   * vision_model
--       The vision agent fallback. Same logic as planner_model:
--       if the per-agent model is missing, this is the fallback.
--   * router_model
--       The master orchestrator's routing-decision model
--       (engine::orchestrator.planning.decisions). The user is
--       right: "router" is the master orchestrator's domain-router,
--       not a per-agent thing.
--
-- We do NOT seed `default_chat_model` (display-only field; the
-- health-summary panel will read the actual resolved model from
-- the orchestrator state instead).
--
-- Values use the sentinel string '__use_first_available__' which
-- resolve_model_name already understands: when the configured value
-- equals this sentinel (or any other unrecognised name), the resolver
-- falls back to the first model in Ollama's /api/tags list, with the
-- existing hard_default as the last-resort name.

INSERT OR IGNORE INTO settings (key, value) VALUES
    ('planner_model',           '__use_first_available__'),
    ('planner_model_fallbacks',  '[]'),
    ('title_generator_model',   '__use_first_available__'),
    ('vision_model',            '__use_first_available__'),
    ('router_model',            '__use_first_available__');

