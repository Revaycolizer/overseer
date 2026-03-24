-- Enhance supervisors table for dynamic orchestration and display configuration
ALTER TABLE supervisors ADD COLUMN fast_path_keywords TEXT;
ALTER TABLE supervisors ADD COLUMN verification_agent_id BIGINT;
ALTER TABLE supervisors ADD COLUMN routing_priority INTEGER DEFAULT 0;
ALTER TABLE supervisors ADD COLUMN hide_evidence BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE supervisors ADD COLUMN report_style TEXT DEFAULT 'detailed';
