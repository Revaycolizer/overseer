-- Migration: 202604052000_kill_search_loops.sql

-- 1. Reinstate strict search loop prevention for ALL supervisors
UPDATE supervisors 
SET instructions = instructions || '

═══ SEARCH LOOP PREVENTION (STRICT) ═══
- NO REPEAT SEARCHES: If a search tool returns "No search results found", DO NOT repeat the exact same search query. 
- MAX ATTEMPTS: Never perform more than 2 searches for the same user request. If you can''t find information after 2 tries (even with different queries), you MUST stop and explain to the user what you tried and why you are stuck.'
WHERE domain IN ('chat', 'helper', 'web', 'stealth');

-- 2. Ensure context consumption is emphasized for Chat too
UPDATE supervisors
SET instructions = instructions || '

═══ SYNTHESIS PROTOCOL ═══
- Prioritize information from KNOWLEDGE_FROM_PREVIOUS_STEPS. If research was done in a previous task, do not search again unless the existing data is insufficient.'
WHERE domain = 'chat';

-- 3. Sync to agents
UPDATE agents SET instructions = (SELECT s.instructions FROM supervisors s WHERE s.domain = agents.domain);
