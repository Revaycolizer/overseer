-- Refine Office Supervisor Routing and Priorities
-- This ensures docx/pdf requests are correctly routed and prioritized.

UPDATE supervisors 
SET fast_path_keywords = 'pdf, word, excel, docx, xlsx, csv, report, export, generate, create, document',
    routing_priority = 25 
WHERE domain = 'office';
