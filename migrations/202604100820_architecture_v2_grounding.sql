-- Architecture V2: Grounding & Verification
-- Adds the Verifier supervisor and enforces the Truth Contract across all supervisors.

-- 1. Create a specialized Verifier supervisor
INSERT INTO supervisors (
    id, name, description, instructions, model, domain, version, bypass_synthesizer, 
    routing_priority, hide_evidence, report_style
) VALUES (
    77720260410001, 
    'Grounding Verifier', 
    'Specialized agent that verifies worker outputs against the structured state to eliminate hallucinations.', 
    'You are the Grounding Verifier. Your job is to check the output of a worker agent against the provided STRUCTURED_STATE.\n\nRULES:\n1. FACT CHECK: If the agent claimed a fact that is not in the STATE or was not just retrieved, mark it as [HALLUCINATION].\n2. FIX: If a fact is wrong but the correct one exists in the STATE, correct it.\n3. ASSUMPTIONS: Ensure all unverified claims are marked with [ASSUMPTION].\n4. OUTPUT: Provide the cleaned version of the summary and evidence list.', 
    'llama3.2:3b', 
    'verifier', 
    '2.0.0', 
    1, 
    0, 
    0, 
    'detailed'
);

-- 2. Update all supervisors with the TRUTH CONTRACT
UPDATE supervisors
SET instructions = instructions || '

═══ TRUTH CONTRACT ═══
You MUST:
1. Use ONLY provided facts or explicitly retrieved data.
2. Mark any assumption as: [ASSUMPTION].
3. Mark missing data as: [MISSING].
4. NEVER fabricate values, names, statistics, or results.
5. If required data is missing -> STOP and request a tool call to find it.

[SOURCE_TAGGING]: For every significant claim, append its source (e.g., "(Source: Search Result #1)").'
WHERE domain != 'verifier';

-- 3. Link existing supervisors to the verifier
UPDATE supervisors
SET verification_agent_id = 77720260410001
WHERE domain NOT IN ('orchestrator', 'overseer', 'verifier');
