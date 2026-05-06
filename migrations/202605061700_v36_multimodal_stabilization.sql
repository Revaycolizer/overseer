-- 202605061700_v36_multimodal_stabilization.sql
-- Refine fast-path routing to prioritize artifact context over broad web research.

-- 1. Remove 'who is' from stealth fast-path (too broad)
UPDATE supervisors 
SET fast_path_keywords = 'research, search, browse, internet, query, latest' 
WHERE domain = 'stealth';

-- 2. Add 'who is' to chat artifact fast-path so it stays in the vision/chat loop if files are attached
UPDATE supervisors 
SET artifact_fast_path_keywords = 'explain,summarize,analyze,read,describe,what is this,tell me about,review,interpret,break down,walk me through,who is,how many,what color,find the'
WHERE domain = 'chat';

-- 3. Ensure 'vision' agent has strict instructions to prune planning artifacts if query is specific
UPDATE agents
SET instructions = instructions || '
═══ V3.6 PROTOCOL ═══
- If the user query is specific (e.g., "who is rated X"), respond ONLY with the findings. 
- Do NOT provide a generic description of the image unless it was explicitly requested.
- Prune all internal reasoning from the response.'
WHERE domain = 'vision';
