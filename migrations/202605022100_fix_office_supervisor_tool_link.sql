-- 202605022100_fix_office_supervisor_tool_link.sql
-- ═══════════════════════════════════════════════════════════════
-- ROOT CAUSE: office supervisor has no agents in supervisor_agents,
-- so get_supervisor_tools() returns empty and the LLM never sees
-- document_writer in the tool list.
-- ═══════════════════════════════════════════════════════════════

-- 1. Link all office-domain agents to the office supervisor
INSERT OR IGNORE INTO supervisor_agents (id, supervisor_id, agent_id)
SELECT abs(random()), s.id, a.id
FROM supervisors s, agents a
WHERE s.domain = 'office' AND a.domain IN ('office', 'office_supervisor', 'text_writer');

-- 2. Ensure document_writer is linked to all those agents
INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id)
SELECT abs(random()), a.id, t.id
FROM agents a, tools t
WHERE a.domain IN ('office', 'office_supervisor', 'text_writer')
  AND t.name = 'document_writer';

-- 3. Harden office supervisor instructions to mandate tool use
UPDATE supervisors
SET instructions = 'You are the OFFICE SUPERVISOR. Your ONLY job is to produce a document file using the document_writer tool.

MANDATORY RULE: You MUST call the document_writer tool. Any response without a tool call is REJECTED.

TOOL CALL FORMAT (use this EXACTLY):
{"observation":"I have the content to write","intent":"Calling document_writer to produce the PDF","tool":"document_writer","input":{"format":"pdf","content":"<full document text here>"}}

CONTENT RULES:
- Write the FULL document text in the "content" field as a plain string.
- Use "\n\n" to separate paragraphs. Do NOT use {title, sections} objects.
- Strip any [E1], [E2] citation markers from the content text.
- Include all research findings provided in the context.'
WHERE domain = 'office';
