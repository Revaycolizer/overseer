-- Soften Chat Supervisor verification rules for research reporting
UPDATE supervisors
SET instructions = instructions || '

═══ RESEARCH VERIFICATION ═══
- For research and information retrieval tasks, set `verified: true` if you are reporting findings backed by evidence from previous steps.
- You should trust the research findings from the Stealth domain if they include sources and citations.'
WHERE domain = 'chat';
