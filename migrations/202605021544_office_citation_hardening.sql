-- ═══════════════════════════════════════════════════════════════
-- Office Domain Hardening: Anti-Lazy Citation Rule
-- ═══════════════════════════════════════════════════════════════

UPDATE supervisors SET instructions = instructions || '

═══ MANDATORY: NO CITATION MARKERS IN DOCUMENTS ═══
When calling tools like "document_writer", YOU MUST expand all citation markers (e.g., [E1], [E2]) into full, descriptive text. 
FORBIDDEN: Never pass "[E1]" or "[E2]" as the "content" for a document section. 
MANDATE: The user expects a full report. Use the evidence provided in the research task to write full paragraphs for each section.'
WHERE domain = 'office';
