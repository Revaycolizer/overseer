-- Database-driven artifact routing: three fixes
-- 1. Add artifact_fast_path_keywords column to supervisors
-- 2. Seed artifact_routing_hint setting
-- 3. Seed artifact_context_header setting

-- Fix 2: Artifact-aware fast-path routing column
ALTER TABLE supervisors ADD COLUMN artifact_fast_path_keywords TEXT;

-- Seed artifact_fast_path_keywords for Chat Supervisor
UPDATE supervisors
SET artifact_fast_path_keywords = 'explain,summarize,analyze,read,describe,what is this,tell me about,review,interpret,break down,walk me through'
WHERE domain = 'chat';

-- Fix 1: Orchestrator routing hint when artifacts are present
INSERT INTO settings (id, key, value) VALUES (
    abs(random()), 'artifact_routing_hint',
    '[ARTIFACTS ATTACHED] The user has selected local file artifacts as context for this request. Route to ''chat'' or ''vision'' domain so the agent can answer from the attached content. Do NOT route to ''stealth'' (search) unless the user explicitly asks to search the web. The attached artifacts contain the information needed to answer.'
) ON CONFLICT(key) DO NOTHING;

-- Fix 3: Artifact context elevation header for supervisor prompts
INSERT INTO settings (id, key, value) VALUES (
    abs(random()), 'artifact_context_header',
    'ATTACHED ARTIFACTS (the user explicitly selected these as context — use them to answer):'
) ON CONFLICT(key) DO NOTHING;
