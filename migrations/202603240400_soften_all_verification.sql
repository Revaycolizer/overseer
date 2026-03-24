-- SOFTEN VERIFICATION RULES for research reporting via the Agents table
-- (The orchestrator uses the agent instructions as the preamble for the supervisor)

-- Stealth Agent
UPDATE agents 
SET instructions = instructions || '

═══ RESEARCH VERIFICATION ═══
- For research tasks, set `verified: true` if you have found relevant evidence that addresses the user query.
- Do NOT require "absolute facts from a system of record"; high-quality web sources are sufficient evidence.'
WHERE domain = 'stealth';

-- Chat Agent 
UPDATE agents
SET instructions = instructions || '

═══ RESEARCH VERIFICATION ═══
- For research and synthesis tasks, set `verified: true` if the reported findings are backed by evidence from previous steps.
- You should trust research findings from the Stealth domain if they include sources and citations.'
WHERE domain = 'chat';
