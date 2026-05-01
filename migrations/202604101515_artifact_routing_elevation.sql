-- Architecture V2: Artifact Routing Elevation
-- Ensuring specialized domains handle artifact generation instead of general chat.

-- 1. Give Office Supervisor high-priority keywords and top-tier routing
UPDATE supervisors
SET fast_path_keywords = 'pdf, docx, xlsx, csv, document, report, generate, export, spreadsheet, excel, word',
    routing_priority = 35 -- Higher than Chat (5) and Stealth (20)
WHERE domain = 'office';

-- 2. Update Master Planner (Orchestrator) to handle decoupled tool failures
UPDATE supervisors
SET instructions = instructions || '

═══ DELEGATION PROTOCOL (V2.4) ═══
1. IF a sub-agent reports that a tool (like "document_writer") is "removed" or "missing":
   - This is an intentional decoupling for modularity.
   - You MUST identify the specialized domain that DOES possess the tool (e.g., "office" for documents) and delegate the specific sub-task to them.
2. DO NOT attempt to write files using the "chat" domain. Always route file generation to "office".
3. If research is needed first, route to "stealth", then pass the research findings to "office" for generation.'
WHERE domain = 'orchestrator';

-- 3. Cleanup Chat Supervisor to be more helpful in redirects
UPDATE supervisors
SET instructions = instructions || '\n\nIMPORTANT: If the user wants a document/PDF, explicitly tell the Master Planner in your "summary" that the task should be delegated to the Office domain.'
WHERE domain = 'chat';
