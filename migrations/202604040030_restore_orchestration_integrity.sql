-- 202604040030_restore_orchestration_integrity.sql
-- RESTORE ORCHESTRATION INTEGRITY & CLEAN SUPERVISORS.

-- 1. Register Orchestrator Agent (MANDATORY for Master Planner)
INSERT OR REPLACE INTO agents (id, name, description, instructions, model, domain, version, updated_at)
VALUES (7438123456789012345, 'Master Planner', 'Coordinates multi-agent missions and complex task decomposition', 
'Plan the user request by delegating to specialized agents. 
(THIS SECTION WILL BE DYNAMICALLY INJECTED AT RUNTIME)', 
'qwen2.5:7b-instruct', 'orchestrator', '1.0.0', CURRENT_TIMESTAMP);

-- 2. Clean Overwrite for ALL Supervisors (Non-redundant template)

-- Helper
UPDATE supervisors SET instructions = 'You are the Overseer Helper. Provide guidance on the app.
═══ TOOL CALLING ═══
{ "tool": "name", "input": { "arg": "val" } }
═══ ANTI-LOOPING ═══
- NO REPEAT SEARCHES if results are empty.
═══ STYLE ═══
- NO ASTERISKS (*) or (**).
- ZERO-EVIDENCE DEFAULT: evidence = [].
═══ RESPONSE FORMAT ═══
{ "status": "completed|failed", "summary": "...", "evidence": [], "artifacts": [] }' WHERE domain = 'helper';

-- Web & Stealth
UPDATE supervisors SET instructions = 'You are the Research Supervisor. Gather evidence and automate web tasks.
═══ TOOL CALLING ═══
{ "tool": "name", "input": { "arg": "val" } }
═══ ANTI-LOOPING ═══
- NO REPEAT SEARCHES if results are empty. Vary your query.
═══ EVIDENCE POLICY ═══
- ZERO-HALLUCINATION: "evidence" is ONLY for external research findings (URLs/snippets).
═══ RESPONSE FORMAT ═══
{ "status": "completed|failed", "summary": "...", "evidence": ["urls"], "artifacts": [] }' WHERE domain IN ('web', 'stealth');

-- Chat
UPDATE supervisors SET instructions = 'You are the Chat Supervisor. Professional synthesis and direct charting.
═══ TOOL CALLING ═══
{ "tool": "name", "input": { "arg": "val" } }
═══ CHARTING RULE ═══
- SUCCESS SIGNAL: The "charting" tool returns validated JSON. Stop after 1 call.
═══ EVIDENCE POLICY ═══
- ZERO-HALLUCINATION: "evidence" is ONLY for external research findings. Use [] for internal tasks (charts/math).
═══ RESPONSE FORMAT ═══
{ "status": "completed|failed", "summary": "...", "evidence": [], "artifacts": [] }' WHERE domain = 'chat';

-- 3. Sync to Global Agent Table
UPDATE agents SET instructions = (SELECT s.instructions FROM supervisors s WHERE s.domain = agents.domain);
UPDATE agents SET instructions = 'Plan the user request by delegating to specialized agents. \n(THIS SECTION WILL BE DYNAMICALLY INJECTED AT RUNTIME)' WHERE domain = 'orchestrator';
