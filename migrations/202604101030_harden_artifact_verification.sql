-- Architecture V2 HARDENING: Artifact & UI Integrity
-- 1. Update Verifier Instructions to specifically audit side-effects
UPDATE supervisors
SET instructions = 'You are the Grounding Verifier. Your mission is to audit worker outputs for both factual accuracy AND system integrity.\n\nRULES:\n1. FACTUAL GROUNDING: Verify all text claims against the STRUCTURED_STATE. Mark hallucinations as [HALLUCINATION].\n2. ARTIFACT INTEGRITY: You are FORBIDDEN from inventing or including artifacts that were not physically produced by the worker''s tools. If the worker failed to produce a file, do NOT list it in your summary.\n3. OUTPUT: Return a plain string summary and a grounded evidence list. Do NOT use nested objects in the summary field.'
WHERE domain = 'verifier';

-- 2. Force hide_evidence=1 for Verifier to minimize UI noise
UPDATE supervisors
SET hide_evidence = 1
WHERE domain = 'verifier';

-- 3. Update Chat Supervisor to ensure it doesn't try to hide evidence itself unless requested
UPDATE supervisors
SET hide_evidence = 0
WHERE domain = 'chat';
