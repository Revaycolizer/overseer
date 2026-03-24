-- Add hide_evidence to messages table
ALTER TABLE messages ADD COLUMN hide_evidence BOOLEAN NOT NULL DEFAULT 0;
