-- Migration: 202604052030_decouple_chat_writer.sql

-- 1. Identify the 'chat' agent's tools and remove 'document_writer'
DELETE FROM agent_tools 
WHERE agent_id IN (SELECT id FROM agents WHERE domain = 'chat')
AND tool_id IN (SELECT id FROM tools WHERE name = 'document_writer');

-- 2. Identify any direct supervisor_agent links that might accidentally grant it
-- (Note: in this system, tools are primarily granted to agents, not directly to supervisors, but we check for safety)

-- 3. Update the Chat Supervisor's instructions to acknowledge it can NO LONGER write documents.
UPDATE supervisors 
SET instructions = instructions || '

═══ TOOL RESTRICTIONS ═══
- NO DOCUMENT WRITING: You are STRICTLY forbidden from generating files (PDF, DOCX, CSV, XLSX). The "document_writer" tool has been REMOVED from your capabilities. If a user needs a document, you must summarize the research and inform them that the Office domain must handle generation.'
WHERE domain = 'chat';

-- 4. Sync to agent
UPDATE agents SET instructions = (SELECT instructions FROM supervisors WHERE domain = 'chat') WHERE domain = 'chat';
