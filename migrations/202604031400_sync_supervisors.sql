-- Synchronize supervisor instructions with the decisiveness rules applied to the agents table.
-- The orchestrator uses the 'supervisors' table for its active instruction sets.

UPDATE supervisors 
SET instructions = (SELECT instructions FROM agents WHERE agents.domain = 'stealth')
WHERE domain = 'stealth';

UPDATE supervisors 
SET instructions = (SELECT instructions FROM agents WHERE agents.domain = 'office')
WHERE domain = 'office';

UPDATE supervisors 
SET instructions = (SELECT instructions FROM agents WHERE agents.domain = 'text_writer')
WHERE domain = 'text_writer';

-- Ensure the models are consistent
UPDATE supervisors SET model = 'qwen2.5:7b-instruct' WHERE domain IN ('stealth', 'office', 'text_writer');
