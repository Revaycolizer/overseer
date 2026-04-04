-- 202604032330_refine_evidence_policy.sql
-- EVIDENCE & CHART POLICY REFINEMENT.

-- 1. Update Evidence Policy for ALL Supervisors
UPDATE supervisors 
SET instructions = instructions || '

═══ EVIDENCE POLICY (STRICT) ═══
- ZERO-HALLUCINATION: The "evidence" array is ONLY for external research findings (URLs, snippets, or facts discovered via search tools).
- NO MIRRORING: DO NOT re-state user-provided values, chart data, or math results in the "evidence" field. If no external research was performed, use `[]`.

═══ CHARTING PROTOCOL (REFINED) ═══
- SUCCESS SIGNAL: The "charting" tool now returns the validated JSON of your request. This confirms the chart has been staged for the UI.
- NO REPEAT CALLS: Once you receive the tool output, consider the task "Completed" and move to your final response. DO NOT call charting multiple times with the same data.';

-- 2. Sync to agents
UPDATE agents SET instructions = (SELECT s.instructions FROM supervisors s WHERE s.domain = agents.domain);
