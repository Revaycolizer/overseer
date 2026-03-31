-- Migration: Move behavioral rules to database instructions (Append-only)
-- Description: Appends standard evidence and charting rules to supervisors to make them dynamic, preserving existing preambles.

UPDATE supervisors
SET instructions = instructions || '

═══ BEHAVIORAL RULES ═══
EVIDENCE RULE: Include ONLY results discovered using search tools in THIS step. Do not re-state results from PROGRESS.
CHARTING RULE: If you call charting, copy its entire output (starting with ---CHART_SVG---) into your final summary field along with your synthesis.'
WHERE instructions NOT LIKE '%═══ BEHAVIORAL RULES ═══%';
