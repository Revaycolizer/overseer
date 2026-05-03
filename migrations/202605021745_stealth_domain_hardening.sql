-- 202605021745_stealth_domain_hardening.sql
-- ═══════════════════════════════════════════════════════════════
-- 1. Register 'web_search' alias for 'stealth_search'
-- ═══════════════════════════════════════════════════════════════
INSERT OR IGNORE INTO tools (id, name, description, language, script_path, code, version)
SELECT 
    abs(random()), 
    'web_search', 
    'Search the web for real-time information, facts, and data. Use this as the primary research tool.', 
    language, 
    script_path, 
    code, 
    version
FROM tools WHERE name = 'stealth_search';

-- ═══════════════════════════════════════════════════════════════
-- 2. Link 'web_search' and 'stealth_search' to all research-capable agents
-- ═══════════════════════════════════════════════════════════════
INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a, tools t
WHERE a.domain IN ('stealth', 'web_supervisor', 'orchestrator', 'chat')
AND t.name IN ('stealth_search', 'web_search');

-- ═══════════════════════════════════════════════════════════════
-- 3. Update Stealth Supervisor Mandate: Enforce Grounding and Tool Use
-- ═══════════════════════════════════════════════════════════════
UPDATE supervisors 
SET instructions = instructions || '

═══ TRUTH CONTRACT V3.3: MANDATORY GROUNDING ═══
1. NO TOOL, NO TRUTH: You are FORBIDDEN from answering research questions without calling a search tool first.
2. CITATION MANDATE: Every factual claim in your summary MUST be followed by a citation marker like [E1], [E2], etc.
3. PHYSICAL REALITY: If you do not call a tool, your response will be REJECTED by the safety layer.
4. EXHAUSTIVE RESEARCH: Do not stop at the first result. Gather multiple data points to ensure the final document is comprehensive.'
WHERE domain = 'stealth';
