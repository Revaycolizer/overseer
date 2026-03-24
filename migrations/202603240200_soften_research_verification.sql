-- Soften Stealth Supervisor and Agent verification rules for more helpful research reporting
UPDATE agents 
SET instructions = REPLACE(instructions, 
    '5. DATA VERIFICATION: Output `done` only when you have a cross-verified or absolute fact from a system of record.',
    '5. DATA VERIFICATION: Set `verified: true` if you have found relevant, high-quality information that addresses the goal, even if it is not an absolute fact from a system of record.'
)
WHERE domain = 'stealth';

UPDATE supervisors
SET instructions = instructions || '

═══ RESEARCH VERIFICATION ═══
- For research and information retrieval tasks, you SHOULD set `verified: true` if you have found relevant evidence that directly addresses the user''s query.
- Do NOT require "absolute facts from a system of record" for general research; high-quality web sources are sufficient evidence.
- If search results were found but you are still refining the answer, you may keep `verified: false` until the final iteration.'
WHERE domain = 'stealth';
