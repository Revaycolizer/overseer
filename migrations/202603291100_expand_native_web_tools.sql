-- Make native web-search/browse tools explicit in the DB and map them to the
-- agents that should be able to use them in workshop/runtime views.

INSERT INTO tools (
    id, name, description, code, language, version,
    creator_id, created_at, updated_at, script_path, required_intent, safety_level
)
SELECT
    1743000000000001,
    'web_search',
    'Search the web and return structured results with titles, URLs, and snippets.',
    '// Native Implementation',
    'rust',
    '1.0.0',
    NULL,
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP,
    NULL,
    NULL,
    'standard'
WHERE NOT EXISTS (
    SELECT 1 FROM tools WHERE name = 'web_search'
);

INSERT INTO tools (
    id, name, description, code, language, version,
    creator_id, created_at, updated_at, script_path, required_intent, safety_level
)
SELECT
    1743000000000002,
    'web_browse',
    'Fetch and clean webpage content from a URL for reading and extraction.',
    '// Native Implementation',
    'rust',
    '1.0.0',
    NULL,
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP,
    NULL,
    NULL,
    'standard'
WHERE NOT EXISTS (
    SELECT 1 FROM tools WHERE name = 'web_browse'
);

UPDATE tools
SET description = 'Launch or focus a local desktop application.',
    code = '// Native Implementation',
    language = 'rust',
    updated_at = CURRENT_TIMESTAMP,
    script_path = NULL,
    safety_level = 'standard'
WHERE name = 'app_launcher';

INSERT INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a
JOIN tools t ON t.name = 'web_search'
WHERE a.domain IN ('stealth', 'web_supervisor')
  AND NOT EXISTS (
      SELECT 1
      FROM agent_tools at
      WHERE at.agent_id = a.id
        AND at.tool_id = t.id
  );

INSERT INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a
JOIN tools t ON t.name = 'web_browse'
WHERE a.domain IN ('stealth', 'web_supervisor')
  AND NOT EXISTS (
      SELECT 1
      FROM agent_tools at
      WHERE at.agent_id = a.id
        AND at.tool_id = t.id
  );

INSERT INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a
JOIN tools t ON t.name = 'app_launcher'
WHERE a.domain IN ('computer', 'desktop', 'orchestrator')
  AND NOT EXISTS (
      SELECT 1
      FROM agent_tools at
      WHERE at.agent_id = a.id
        AND at.tool_id = t.id
  );
