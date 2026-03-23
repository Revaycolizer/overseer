-- Cleanup Migration: Drops all data from tables (excluding conversations)
-- Runs before the full data seeding migration.

PRAGMA foreign_keys = OFF;

DELETE FROM execution_events;
DELETE FROM execution_runs;
DELETE FROM attachments;
DELETE FROM messages;
DELETE FROM workflow_step_runs;
DELETE FROM workflow_runs;
DELETE FROM workflow_steps;
DELETE FROM agent_planners;
DELETE FROM supervisor_agents;
DELETE FROM agent_tools;
DELETE FROM workflows;
DELETE FROM planners;
DELETE FROM supervisors;
DELETE FROM tools;
DELETE FROM agents;
DELETE FROM settings;
DELETE FROM versions;

PRAGMA foreign_keys = ON;
