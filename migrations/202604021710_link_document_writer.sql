-- Link document_writer to text_writer and office agents
INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a, tools t
WHERE a.domain IN ('text_writer', 'office', 'office_supervisor', 'computer', 'desktop', 'neontune')
AND t.name = 'document_writer';
