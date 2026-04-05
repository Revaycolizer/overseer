-- 202604041835_link_chat_document_writer.sql
-- Grant broad access to document_writer and charting as a safety net.
-- Based on the schema: supervisors → supervisor_agents → agents ↔ agent_tools ↔ tools

-- 1. Grant to agents (this is where the tools are linked)
INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a, tools t
WHERE a.domain IN ('chat', 'office', 'text_writer', 'overseer', 'helper')
AND t.name IN ('document_writer', 'charting');

-- 2. Ensure Chat Agent and Supervisor are linked (if they aren't already)
-- This facilitates the join in runner.rs
INSERT OR IGNORE INTO supervisor_agents (id, supervisor_id, agent_id)
SELECT abs(random()), s.id, a.id
FROM supervisors s, agents a
WHERE s.domain = 'chat' AND a.domain = 'chat';
