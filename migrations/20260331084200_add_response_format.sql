-- Migration: Add response_format to supervisors, agents, and planners
-- and standardize all existing modern supervisors to use the JSON schema.

-- 1. Add column to supervisors
ALTER TABLE supervisors ADD COLUMN response_format TEXT DEFAULT 'json';

-- 2. Add column to agents
ALTER TABLE agents ADD COLUMN response_format TEXT DEFAULT 'json';

-- 3. Add column to planners 
-- Note: some versions of this DB have planners, some don't. Using PRAGMA logic or just standard SQL.
ALTER TABLE planners ADD COLUMN response_format TEXT DEFAULT 'json';

-- 4. Standardize existing supervisor instructions for JSON-based domains
-- Only update if the instructions don't already mention 'status' or 'evidence'
UPDATE supervisors 
SET instructions = instructions || '
═══ RESPONSE FORMAT (MANDATORY JSON ONLY) ═══
Return ONLY this JSON schema for your final response:
{
  "status": "completed|unverified|failed",
  "summary": "concise overview of the result",
  "evidence": [...result objects with title, url, snippet...],
  "artifacts": ["list of file paths created"],
  "verified": true
}'
WHERE response_format = 'json' 
  AND (instructions NOT LIKE '%"status"%' OR instructions NOT LIKE '%"evidence"%');

-- 5. Standardize Computer Operator agent for JSON output
UPDATE agents
SET instructions = instructions || ' Always respond with the structured JSON result schema for your final answer.'
WHERE domain = 'computer' AND instructions NOT LIKE '%JSON%';
