-- 202604032230_universal_search_loop_fix.sql
-- Refine ALL Supervisors with Search Loop Prevention.

UPDATE supervisors 
SET instructions = instructions || '

═══ CORE RULES (REINFORCED) ═══
- NO SEARCH LOOPING: If a search tool (stealth_search, web_search, etc.) returns "No search results found", you MUST NOT repeat the exact same search query or tool call again. 
- FAILURE HANDLING: If a search fails, you must either vary your search term significantly or conclude that the information is unavailable. NEVER perform more than 2 searches for the same user query.
- SYNTHESIS: Your mission is to synthesize information from previous steps into a clean, direct results summary.';

-- Sync to agents
UPDATE agents SET instructions = (SELECT s.instructions FROM supervisors s WHERE s.domain = agents.domain);
