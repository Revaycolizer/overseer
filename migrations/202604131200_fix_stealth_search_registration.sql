-- Ensure stealth_search exists in tools
INSERT OR IGNORE INTO tools (id, name, description, language, script_path, code, version)
VALUES (999001, 'stealth_search', 'Search the web anonymously and return structured findings.', 'rust', '', '// Native Implementation', '1.0.0');

-- Link stealth_search to the stealth supervisor
INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id)
SELECT (ABS(RANDOM()) % 9223372036854775807), a.id, t.id
FROM agents a
JOIN tools t ON t.name = 'stealth_search'
WHERE a.domain = 'stealth' AND NOT EXISTS (
    SELECT 1 FROM agent_tools at2 WHERE at2.agent_id = a.id AND at2.tool_id = t.id
);

-- Link stealth_search to the chat supervisor
INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id)
SELECT (ABS(RANDOM()) % 9223372036854775807), a.id, t.id
FROM agents a
JOIN tools t ON t.name = 'stealth_search'
WHERE a.domain = 'chat' AND NOT EXISTS (
    SELECT 1 FROM agent_tools at2 WHERE at2.agent_id = a.id AND at2.tool_id = t.id
);

-- Ensure web_search is also linked to stealth (redundancy)
INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id)
SELECT (ABS(RANDOM()) % 9223372036854775807), a.id, t.id
FROM agents a
JOIN tools t ON t.name = 'web_search'
WHERE a.domain = 'stealth' AND NOT EXISTS (
    SELECT 1 FROM agent_tools at2 WHERE at2.agent_id = a.id AND at2.tool_id = t.id
);
