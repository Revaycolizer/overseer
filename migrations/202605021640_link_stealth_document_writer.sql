-- Corrected migration to link document_writer to Stealth domain via agent_tools.
-- This ensures the Stealth Supervisor can access the tool through its underlying agents.

INSERT INTO agent_tools (id, agent_id, tool_id)
SELECT 
    (COALESCE((SELECT MAX(id) FROM agent_tools), 0) + abs(random()) % 1000 + 1),
    sa.agent_id, 
    t.id
FROM supervisor_agents sa, tools t, supervisors s
WHERE s.domain = 'stealth'
  AND sa.supervisor_id = s.id
  AND t.name = 'document_writer'
  AND NOT EXISTS (
    SELECT 1 FROM agent_tools at 
    WHERE at.agent_id = sa.agent_id AND at.tool_id = t.id
  );
