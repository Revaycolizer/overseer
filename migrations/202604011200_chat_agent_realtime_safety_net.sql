-- 1. Link Research Tools to Chat Domain
-- Using subqueries to be ID-agnostic, though I have the IDs for reference.
INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id)
SELECT ABS(RANDOM()), a.id, t.id
FROM agents a, tools t
WHERE a.domain = 'chat' AND t.name IN ('stealth_search', 'web_search');

-- 2. Refine Stealth Supervisor Keywords to prevent hijacking basic conversational queries
UPDATE supervisors 
SET fast_path_keywords = 'research, search, browse, latest, documentation, news, how to, guide, tutorial, github, release, update, terminal, command, shell, extract, summarize, evidence, info, explore'
WHERE domain = 'stealth';

-- 3. Update Behavioral Instructions for Chat and Stealth Supervisors
-- We append the dynamic/static and evidence suppression rules.
UPDATE supervisors 
SET instructions = instructions || '

═══ KNOWLEDGE & EVIDENCE RULES ═══
- KNOWLEDGE FIRST: If the user asks for static, general knowledge (e.g., historical dates, common facts), answer immediately from your internal knowledge WITHOUT using research tools. Use research tools ONLY for dynamic, recent, or specialized information.
- REAL-TIME AUTONOMY: If you identify that a query refers to a current event, dynamic data, or information that changes over time, and your internal knowledge is insufficient or potentially outdated, you ARE authorized to use the search tools.
- EVIDENCE SUPPRESSION: Include the "evidence" JSON field ONLY if the user explicitly asked for "sources", "links", "citations", "evidence", or "verification". Otherwise, ALWAYS return an empty array [] for "evidence".'
WHERE domain IN ('chat', 'stealth');

-- 4. Update the Master Orchestrator (Planner) with clear Dynamic vs Static examples
UPDATE agents
SET instructions = instructions || '

═══ DYNAMIC VS STATIC ROUTING ═══
- STATIC: "What is the capital of France?" -> Use "chat".
- DYNAMIC: "What is the current price of Bitcoin?" -> Use "stealth" for research, then "chat" for synthesis.
- AMBIGUOUS: "Who is the CEO of Apple?" -> Use "stealth" if you sense your training data might be stale, else "chat".'
WHERE domain = 'orchestrator';
