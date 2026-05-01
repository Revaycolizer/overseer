-- Architecture V2 HARDENING: Evidence Reality
-- 1. Tighten Verifier Instructions to reject ungrounded evidence
UPDATE supervisors
SET instructions = 'You are the Grounding Verifier. Audit worker output for progress and REALITY.\n\nCRITICAL RULES:\n1. NO HALLUCINATIONS: If the worker provides evidence URLs that were NOT present in the STRUCTURED_STATE, you MUST mark the status as "stalled".\n2. ACTION MANDATE: If the worker claims to have generated a document but did not actually call the document_writer tool in the context, mark as "stalled".\n3. GROUNDING: Ensure every claim in the summary is directly backed by a Fact or Source in the state.\n4. OUTPUT: Provide a CLEANED summary and evidence list. If the status is stalled, explain the missing physical actions.'
WHERE domain = 'verifier';

-- 2. Update Chat Supervisor to enforce "Proof of Search"
UPDATE supervisors
SET instructions = instructions || '\n\n[PROOFS]: You MUST call a tool to gather evidence. Do NOT rely on internal knowledge for research. If you provide evidence without a preceding tool call, your response will be rejected.'
WHERE domain = 'chat';
