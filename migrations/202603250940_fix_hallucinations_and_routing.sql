-- ═══════════════════════════════════════════════════════════════
-- Fix Hallucinations and Routing Adjustments
-- ═══════════════════════════════════════════════════════════════

-- 1. Routing Adjustment: Move research-heavy keywords to Stealth
UPDATE supervisors SET fast_path_keywords = 'research, search, browse, internet, query, latest, which is, who is, explain, what is, how, why'
WHERE domain = 'stealth';

UPDATE supervisors SET fast_path_keywords = 'hello, hi, help, what if'
WHERE domain = 'chat';

-- 2. Stricter Anti-Hallucination Instructions for all core domains
UPDATE supervisors SET instructions = instructions || '

═══ ANTI-HALLUCINATION — STRICT ═══
NEVER invent URLs, search results, or citations. 
If you did not use a search tool or if the information is not in your context, do not provide it as evidence. 
If an image or diagram is not found in your context, state that you cannot provide it rather than using placeholders like example.com. 
Admit if you cannot fulfill a specific request for a visual or a link.'
WHERE domain IN ('chat', 'stealth', 'web', 'desktop', 'computer');
