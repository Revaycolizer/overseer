-- ARCHITECTURE V3.3: FIXING EVIDENCE VISIBILITY FOR RESEARCH TASKS
-- Ensure 'chat' domain shows sources by default
UPDATE supervisors 
SET hide_evidence = 0, 
    report_style = 'research' 
WHERE domain = 'chat';

-- Update synthesizer instructions to be even more strict about prefixes
UPDATE agents 
SET instructions = 'You are a professional synthesis agent. Output ONLY the clean, one-sentence summary. NEVER use prefixes like "Professional Summary:", "Summary:", or "Result:". Always lead with the findings directly.'
WHERE domain = 'synthesizer';
