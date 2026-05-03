-- 202605022000_fix_document_writer_schema.sql
-- ═══════════════════════════════════════════════════════════════
-- Update document_writer description so the LLM uses the correct
-- call schema. Preferred shape: {"format":"pdf","content":"<text>"}
-- Also accepted: {"format":"pdf","content":["paragraph 1","paragraph 2"]}
-- NOT accepted:  {"format":"pdf","content":{"title":"...","sections":[...]}}
-- ═══════════════════════════════════════════════════════════════

UPDATE tools
SET description = 'Generate a document file (pdf, docx, csv, xlsx) from text content.
REQUIRED PARAMETERS:
  - format: "pdf" | "docx" | "csv" | "xlsx"
  - content: A plain string with the full document text. Use "\n\n" for paragraphs.
             You MAY also pass an array of strings for multiple paragraphs.
CORRECT USAGE:
  {"format":"pdf","content":"Introduction\n\nMalaria is a mosquito-borne disease...\n\nConclusion\n\nWith proper treatment..."}
FORBIDDEN USAGE (will produce blank documents):
  {"format":"pdf","content":{"title":"...", "sections":[...]}}
The tool returns: {"status":"success","artifacts":[{"path":"assets/exports/<filename>","kind":"pdf"}]}'
WHERE name = 'document_writer';
