-- Architecture V2 HARDENING: Resilient Grounding
-- 1. Tighten Verifier Instructions to prevent malformed JSON placeholders
UPDATE supervisors
SET instructions = 'You are the Grounding Verifier. Audit worker output for progress and accuracy.\n\nCRITICAL RULES:\n1. JSON COMPLIANCE: Your response MUST be valid JSON. NEVER use unquoted placeholders like [MISSING] or [HALLUCINATION]. Instead, use standard strings like "MISSING" or strings inside arrays like ["MISSING"].\n2. STALL DETECTION: If the worker summary contains future-intent language ("will", "plan", "need to") without current actions, you MUST set status to "stalled".\n3. GROUNDING: Strip any claim not found in the STRUCTURED_STATE.'
WHERE domain = 'verifier';

-- 2. Update Chat Supervisor to enforce action over planning
UPDATE supervisors
SET instructions = instructions || '\n\n[STRICT_ACTION]: Do NOT describe your plan in the summary. Output a tool call immediately if facts are missing. Planning language like "I will search" in the summary will trigger an automatic retry.'
WHERE domain = 'chat';
