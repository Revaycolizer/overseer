-- 202605042220_cache_attachment_content.sql
-- Add extracted_content to attachments table to persist multimodal perception results.

ALTER TABLE attachments ADD COLUMN extracted_content TEXT;
