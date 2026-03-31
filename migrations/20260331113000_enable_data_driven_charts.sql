-- Finalize Data-Driven Charting Protocol
-- This migration standardizes the 'charts' field across all supervisor instructions.

-- 1. Standardize Desktop Supervisor (Structured JSON mode)
UPDATE supervisors 
SET instructions = REPLACE(
    REPLACE(
        instructions, 
        '"verified": true', 
        '"charts": [{"chart_type": "bar|line|pie", "title": "...", "labels": ["A", "B"], "data": [10, 20]}], "verified": true'
    ),
    'CHARTING RULE: If you call charting, copy its entire output (starting with ---CHART_SVG---) into your final summary field along with your synthesis.',
    'CHARTING RULE: If you want to visualize data, return a JSON object in the "charts" field: {"chart_type": "bar|line|pie", "title": "...", "labels": ["A", "B"], "data": [10, 20]}. The system will render it for you. Do NOT copy SVG code.'
)
WHERE name = 'Desktop Supervisor';

-- 2. Standardize other supervisors (Behavioral Rules update)
UPDATE supervisors 
SET instructions = REPLACE(
    instructions,
    'CHARTING RULE: If you call charting, copy its entire output into your final summary.',
    'CHARTING RULE: If you want to visualize data, return a JSON object in the "charts" field: {"chart_type": "bar|line|pie", "title": "...", "labels": ["A", "B"], "data": [10, 20]}. The system will render it for you. Do NOT copy SVG code.'
)
WHERE name != 'Desktop Supervisor';

-- 3. Ensure "Chat Supervisor" also has the schema scaffold if it's missing
UPDATE supervisors
SET instructions = instructions || '
═══ RESPONSE FORMAT (MANDATORY JSON ONLY) ═══
Return ONLY this JSON schema for your final response:
{
  "status": "completed|unverified|failed",
  "summary": "concise overview of the result",
  "evidence": [...result objects with title, url, snippet...],
  "charts": [{"chart_type": "bar|line|pie", "title": "...", "labels": ["A", "B"], "data": [10, 20]}],
  "artifacts": ["list of file paths created"],
  "verified": true
}'
WHERE name = 'Chat Supervisor' AND instructions NOT LIKE '%MANDATORY JSON ONLY%';
