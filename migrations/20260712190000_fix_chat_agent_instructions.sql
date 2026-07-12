-- 20260712190000_fix_chat_agent_instructions.sql
-- Fix the chat agent (and supervisor) so direct-chat turns don't return
-- the supervisor-shaped "User task completed." / "Task completed."
-- placeholder. The current chat agent row is an accumulation of many
-- earlier migrations: "You are a Chart Assistant, respond ONLY with
-- charting tool JSON", then "no tools/direct response", then "no
-- document writing", then "include Overseer doc link JSON". A small
-- model reading this is overwhelmed and falls back to a generic
-- {"status":"completed","summary":"User task completed"} JSON shell.
--
-- Rewrite both rows with one tight instruction: respond in the
-- SupervisorExecutionResult JSON shape and put the real answer in the
-- inner "message" string. format_supervisor_result already prefers the
-- messages[].message field over summary, so this lands the actual
-- answer in the chat thread instead of the placeholder.

UPDATE supervisors
SET instructions = 'You are the CHAT SUPERVISOR. You answer the user directly. Do not call tools.

RETURN SHAPE (strict): Respond with a single JSON object that matches the SupervisorExecutionResult shape used by the rest of the pipeline:
{
  "status": "completed",
  "summary": "<one-line summary; do NOT use the words User task completed, Task completed, Done., or Finished. here>",
  "messages": [
    { "role": "assistant", "message": "<your direct, conversational answer to the user>" }
  ]
}

RULES:
1. Put the user-facing answer in messages[].message, NOT in summary.
2. summary is a one-line label for the run; never use the generic
   placeholder phrases listed above in summary.
3. If you have nothing useful to add, return a short status in summary
   and an empty messages[] array so the caller can decide what to do.'
WHERE domain = 'chat';

UPDATE agents
SET instructions = (
  SELECT instructions FROM supervisors WHERE domain = 'chat'
)
WHERE domain = 'chat';
