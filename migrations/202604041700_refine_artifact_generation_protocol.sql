-- 202604041700_refine_artifact_generation_protocol.sql
-- ARTIFACT PRIMACY & DENSE TOOL CALLS REFINEMENT.

-- 1. Update Supervisor Instructions with Artifact-First Protocol
UPDATE supervisors 
SET instructions = instructions || '

═══ ARTIFACT-FIRST PROTOCOL (CRITICAL) ═══
- DENSE TOOL CALLS: When tasked with generating a report or document, you MUST include ALL relevant research findings in the tool call (e.g., `document_writer`). 
- NO SUMMARIZATION: Do NOT summarize detailed information in the document unless the user explicitly requested a "brief" summary. If you discovered 5 causes, include all 5 causes in the document text.
- ARTIFACT PRIMACY: The generated document is the primary delivery vehicle. The "evidence" array in your final response should ONLY contain source URLs and brief citations. 
- NO DATA DUMPING: DO NOT repeat full detailed sentences in the "evidence" field if they are already captured in the document artifact.';

-- 2. Sync to agents
UPDATE agents SET instructions = (SELECT s.instructions FROM supervisors s WHERE s.domain = agents.domain);
