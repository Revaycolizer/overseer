-- Migration: 202604051800_enforce_artifact_silence.sql

-- 1. Set hide_evidence = 1 for delivery domains to reduce UI noise
UPDATE supervisors SET hide_evidence = 1 WHERE domain IN ('chat', 'office', 'text_writer', 'helper');

-- 2. Update 'chat' supervisor instructions to be silent when artifacts are present
UPDATE supervisors 
SET instructions = instructions || '

═══ ARTIFACT-SILENCE RULE (CRITICAL) ═══
- If a FILE (PDF, DOCX, XLSX, etc.) was created in a previous step (check EXECUTION_HISTORY), DO NOT repeat the detailed findings from that file in your summary. 
- Your job is simply to confirm the artifact is ready and provide a 1-sentence professional greeting. 
- NEVER re-list symptoms, causes, or data in your "summary" if they are already in the generated document.'
WHERE domain = 'chat';

-- 3. Update 'office' supervisor instructions to mandate context consumption
UPDATE supervisors
SET instructions = instructions || '

═══ DATA-DRIVEN CONTENT RULE (CRITICAL) ═══
- You MUST consume all facts from KNOWLEDGE_FROM_PREVIOUS_STEPS to populate the document. 
- If Task #1 found "Causes" and "Symptoms", you MUST include sections for "Causes" and "Symptoms" in the generated file with the specific details from Task #1. 
- DO NOT OMIT DATA. If the research step provided 10 facts, include 10 facts in the document.'
WHERE domain = 'office';
