-- Map native device control tools to the web_supervisor agent
INSERT INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a
JOIN tools t ON t.name IN ('os_tool', 'app_launcher')
WHERE a.domain = 'web_supervisor'
  AND NOT EXISTS (
      SELECT 1
      FROM agent_tools at
      WHERE at.agent_id = a.id
        AND at.tool_id = t.id
  );

