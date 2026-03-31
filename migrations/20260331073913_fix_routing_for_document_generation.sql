-- Add migration script here
UPDATE supervisors 
SET fast_path_keywords = 'pdf, word, excel, docx, xlsx, csv, report, export, document',
    routing_priority = 25 
WHERE domain = 'office';