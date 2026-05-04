-- Update supervisors with dynamic synthesis flags
-- This removes hardcoding from the Rust codebase and allows for a scalable agent ecosystem

-- 1. Helper / Support
UPDATE supervisors 
SET report_style = 'help', 
    hide_evidence = 1 
WHERE domain = 'helper';

-- 2. Office / Document Generation
UPDATE supervisors 
SET report_style = 'delivery', 
    hide_evidence = 1 
WHERE domain = 'office';

-- 3. Research / Stealth
UPDATE supervisors 
SET report_style = 'research', 
    hide_evidence = 0 
WHERE domain = 'stealth' OR domain = 'web';

-- 4. Chat / General
UPDATE supervisors 
SET report_style = 'detailed', 
    hide_evidence = 1 
WHERE domain = 'chat';

-- 5. Computer / Desktop (Usually want to hide evidence unless research-focused)
UPDATE supervisors 
SET report_style = 'detailed', 
    hide_evidence = 1 
WHERE domain = 'computer' OR domain = 'desktop';
