-- Set skip_fast_path_if_artifacts = 1 for Chat Supervisor
UPDATE supervisors 
SET skip_fast_path_if_artifacts = 1 
WHERE domain = 'chat';
