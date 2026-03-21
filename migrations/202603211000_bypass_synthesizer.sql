-- Add bypass_synthesizer flag to supervisors
-- When true, the orchestrator returns this supervisor's result directly,
-- skipping the Synthesizer LLM entirely.
ALTER TABLE supervisors ADD COLUMN bypass_synthesizer BOOLEAN NOT NULL DEFAULT 0;

-- The 'chat' supervisor produces charts — enable bypass for it by default
UPDATE supervisors SET bypass_synthesizer = 1 WHERE domain = 'chat';
