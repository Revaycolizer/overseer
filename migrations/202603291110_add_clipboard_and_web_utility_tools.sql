-- Add more explicit native utility tools for web extraction and clipboard work.

INSERT INTO tools (
    id, name, description, code, language, version,
    creator_id, created_at, updated_at, script_path, required_intent, safety_level
)
SELECT
    1743000000000003,
    'web_extract',
    'Fetch a web page and return structured cleaned content with title and status.',
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
    SELECT 1 FROM tools WHERE name = 'web_extract'
);

INSERT INTO tools (
    id, name, description, code, language, version,
    creator_id, created_at, updated_at, script_path, required_intent, safety_level
)
SELECT
    1743000000000004,
    'web_links',
    'Fetch a web page and extract a structured list of links from it.',
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
    SELECT 1 FROM tools WHERE name = 'web_links'
);

INSERT INTO tools (
    id, name, description, code, language, version,
    creator_id, created_at, updated_at, script_path, required_intent, safety_level
)
SELECT
    1743000000000005,
    'clipboard_read',
    'Read text from the system clipboard.',
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
    SELECT 1 FROM tools WHERE name = 'clipboard_read'
);

INSERT INTO tools (
    id, name, description, code, language, version,
    creator_id, created_at, updated_at, script_path, required_intent, safety_level
)
SELECT
    1743000000000006,
    'clipboard_write',
    'Write text to the system clipboard.',
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
    SELECT 1 FROM tools WHERE name = 'clipboard_write'
);

INSERT INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a
JOIN tools t ON t.name IN ('web_extract', 'web_links')
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
JOIN tools t ON t.name IN ('clipboard_read', 'clipboard_write')
WHERE a.domain IN ('computer', 'desktop', 'orchestrator')
  AND NOT EXISTS (
      SELECT 1
      FROM agent_tools at
      WHERE at.agent_id = a.id
        AND at.tool_id = t.id
  );
