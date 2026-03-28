-- Add skip_fast_path_if_artifacts to supervisors
ALTER TABLE supervisors ADD COLUMN skip_fast_path_if_artifacts BOOLEAN NOT NULL DEFAULT 0;
