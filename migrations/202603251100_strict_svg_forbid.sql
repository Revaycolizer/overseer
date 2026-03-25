-- ═══════════════════════════════════════════════════════════════
-- Strict No-Code Diagram Rule
-- ═══════════════════════════════════════════════════════════════

-- 1. Ensure routing keywords are correct (just in case)
UPDATE supervisors SET fast_path_keywords = 'research, search, browse, internet, query, latest, which is, who is, explain, what is, how, why'
WHERE domain = 'stealth';

UPDATE supervisors SET fast_path_keywords = 'hello, hi, help, what if'
WHERE domain = 'chat';

-- 2. Add strict anti-SVG/HTML diagram rules
UPDATE supervisors SET instructions = instructions || '

═══ NO-CODE DIAGRAMS — CRITICAL ═══
NEVER generate raw SVG, HTML, or code snippets to represent diagrams or components. 
Do NOT "draw" diagrams using markdown, code blocks, or SVG. 
Only use valid image URLs [IMAGE: URL] found in your research or provided in context. 
If no image or diagram is found, simply state that a visual is not available and provide a textual description instead.' 
WHERE domain IN ('chat', 'stealth', 'web', 'desktop', 'computer');
