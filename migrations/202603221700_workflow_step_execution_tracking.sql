ALTER TABLE workflow_step_runs
ADD COLUMN execution_run_id BIGINT;

ALTER TABLE workflow_step_runs
ADD COLUMN finished_at DATETIME;
