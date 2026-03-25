-- ═══════════════════════════════════════════════════════════════
-- Move Hard-coded Instructions from Dispatcher to Database
-- ═══════════════════════════════════════════════════════════════

-- 1. Chat Supervisor: Source Preservation and SVG Forbid (Consolidated)
UPDATE supervisors SET instructions = instructions || '

═══ SOURCE PRESERVATION — CRITICAL ═══
- You MUST preserve all URLs, original Titles, and Snippets from previous research steps in your own ''evidence'' list.
- EVERY evidence item should be a complete JSON-like object: {"title": "...", "url": "...", "snippet": "..."}
- Ensure findings are reported as ''verified: true'' if they are backed by sources in the PROGRESS section.
- NEVER provide examples of HTML, SVG, or code unless explicitly asked for ''code'' or ''markup''.'
WHERE domain = 'chat';

-- 2. Stealth Supervisor: Research Summary Rule
UPDATE supervisors SET instructions = instructions || '

═══ RESEARCH SUMMARY RULE ═══
- If you find evidence, you MUST provide a concise overview in the ''summary'' field.
- NEVER return an empty summary if evidence was found.'
WHERE domain = 'stealth';
