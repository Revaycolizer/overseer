-- Kernel filesystem-write capability: register the native `write_file` tool and
-- grant it to the file-capable domains. It is a typed FilesystemWrite action,
-- governed by the kernel (approval-required by default) — never a shell command.

INSERT INTO tools (
    id, name, description, code, language, version,
    creator_id, created_at, updated_at, script_path, required_intent, safety_level
)
SELECT
    1752900000000001,
    'write_file',
    'Write text content to a file at an absolute path (creates parent directories). Input: {"path": "...", "content": "..."}. Governed by the kernel as a filesystem write; requires human approval.',
    '// Native Implementation',
    'rust',
    '1.0.0',
    NULL,
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP,
    NULL,
    NULL,
    'high'
WHERE NOT EXISTS (
    SELECT 1 FROM tools WHERE name = 'write_file'
);

INSERT INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a
JOIN tools t ON t.name = 'write_file'
WHERE a.domain IN ('computer', 'desktop', 'office', 'text_writer', 'orchestrator')
  AND NOT EXISTS (
      SELECT 1
      FROM agent_tools at
      WHERE at.agent_id = a.id
        AND at.tool_id = t.id
  );
