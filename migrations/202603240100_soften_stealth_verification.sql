-- Soften Stealth Agent verification and ensure marker usage
UPDATE agents 
SET instructions = instructions || '

═══ REPORTING PROTOCOL ═══
- If you find any relevant data or high-quality sources, you SHOULD report them as evidence, even if you cannot fully verify the total "top three" or exact facts requested.
- When returning search results, ALWAYS include the `---SEARCH_RESULTS---` marker at the end of your report, followed by the JSON from the tool. This ensures the user sees the rich result cards.'
WHERE domain = 'stealth';

-- Soften Stealth Supervisor verification
UPDATE supervisors
SET instructions = REPLACE(instructions, 'The Chat Supervisor handles visualization; your job is data integrity.', 'The Chat Supervisor handles visualization; your job is data integrity. If search results were found, ALWAYS include the `---SEARCH_RESULTS---` marker with the JSON array in your output to enable the rich interactive view.')
WHERE domain = 'stealth';
