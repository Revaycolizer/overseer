-- 202605022130_unlink_stealth_document_writer.sql
-- ═══════════════════════════════════════════════════════════════
-- ROOT CAUSE: Stealth agent was incorrectly linked to document_writer,
-- causing it to try and generate the PDF during the research phase.
-- This resulted in a redundant "double call" and clutter.
-- ═══════════════════════════════════════════════════════════════

DELETE FROM agent_tools 
WHERE tool_id IN (SELECT id FROM tools WHERE name = 'document_writer')
  AND agent_id IN (
    SELECT sa.agent_id 
    FROM supervisor_agents sa, supervisors s 
    WHERE sa.supervisor_id = s.id AND s.domain = 'stealth'
  );
