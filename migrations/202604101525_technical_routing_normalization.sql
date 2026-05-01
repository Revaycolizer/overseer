-- Architecture V2: Technical Routing Normalization
-- Eliminating keyword length bias and conversational hijacking.

-- 1. Prune conversational fillers from Chat Supervisor
-- Moving research keywords to 'Stealth' or 'Web' domains.
UPDATE supervisors
SET fast_path_keywords = 'hello, hi, help, explain, what if, who are you, tell me about, how can you help, greeting'
WHERE domain = 'chat';

-- 2. Refine Stealth Supervisor to own research tokens
UPDATE supervisors
SET fast_path_keywords = 'research, search, browse, internet, query, latest, which is, who is, tell me about malaria, symptoms, causes',
    routing_priority = 25
WHERE domain = 'stealth';

-- 3. Elevate Office to dominate 'Artifact' tokens
UPDATE supervisors
SET fast_path_keywords = 'pdf, generate pdf, create document, report, excel, docx, document, file, export, malaria pdf',
    routing_priority = 40 -- Absolute highest for artifact tasks
WHERE domain = 'office';

-- 4. Master Planner: Enforce Multi-Step Logic for Documents
UPDATE supervisors
SET instructions = instructions || '

═══ ARTIFACT MANDATE (V2.5) ═══
- For any request involving files (PDF, Doc, XLSX): You MUST use a multi-step plan.
- Step 1: Research (domain: stealth) to gather facts.
- Step 2: Generation (domain: office) to create the file.
- DO NOT attempt to combine these into a single "chat" step.'
WHERE domain = 'orchestrator';
