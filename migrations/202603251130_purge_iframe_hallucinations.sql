-- Migration: Purge iframe/HTML hallucinations and enforce standard Markdown
-- Date: 2026-03-25 11:30

-- 1. Purge outdated "Visualization Rule" and "JSON-like" requirements from ALL agents
UPDATE agents SET instructions = REPLACE(instructions, 'VISUALIZATION RULE: If the research findings contain an HTML or SVG document (e.g., a chart), you MUST include that raw HTML/SVG document at the end of your response without any markdown wrapping. Do not strip or summarize the technical HTML code if it is intended for visualization.', '');
UPDATE agents SET instructions = REPLACE(instructions, 'EVERY evidence item should be a complete JSON-like object: {"title": "...", "url": "...", "snippet": "..."}', 'Every evidence item must be a standard clickable Markdown link: [Title](URL)');

-- 2. Purge SVG/HTML requirements from supervisors
UPDATE supervisors SET instructions = REPLACE(instructions, 'NEVER generate raw SVG, HTML, or code snippets to represent diagrams or components.', '');
UPDATE supervisors SET instructions = REPLACE(instructions, 'Do NOT "draw" diagrams using markdown, code blocks, or SVG.', '');

-- 3. Enforce Standard Markdown Rule across all components
UPDATE supervisors SET instructions = instructions || '

═══ STANDARD MARKDOWN RULE — CRITICAL ═══
- ALL links MUST be standard clickable Markdown: [Title](URL).
- ALL images MUST be standard Markdown: ![Alt](URL).
- NEVER use <iframe>, raw HTML, SVG, or JSON wrappers for evidence or diagrams.
- Use [IMAGE: URL] markers from your research to provide rich visual evidence.';

UPDATE agents SET instructions = instructions || '

═══ STANDARD MARKDOWN RULE — CRITICAL ═══
- ALL links MUST be standard clickable Markdown: [Title](URL).
- ALL images MUST be standard Markdown: ![Alt](URL).
- NEVER use <iframe>, raw HTML, SVG, or JSON wrappers for evidence or diagrams.';

-- 4. Specifically ensure the Synthesizer is clean
UPDATE agents SET instructions = instructions || '
- Your final output must be 100% human-readable Markdown. 
- NO technical logs, NO internal step markers, NO iframes.'
WHERE domain = 'synthesizer';
