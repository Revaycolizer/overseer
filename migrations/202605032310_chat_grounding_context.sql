-- 202605032310_chat_grounding_context.sql
-- Add Overseer brand context to the Chat Supervisor to prevent hallucinations during general chat.

UPDATE supervisors 
SET instructions = instructions || '\n\n═══ OVERSEER BRAND & DOCUMENTATION (STRICT) ═══\n- APPLICATION: Overseer\n- OFFICIAL DOCUMENTATION: https://overseer.aimsoftwares.com/docs\n- RULE: If asked about the application, navigation, or documentation, you MUST use the link above. Never hallucinate other links.\n- EVIDENCE: If mentioning the documentation, include this JSON in your evidence: {"title": "Overseer Docs", "url": "https://overseer.aimsoftwares.com/docs", "snippet": "Official documentation."}'
WHERE domain = 'chat';

UPDATE agents SET instructions = (SELECT instructions FROM supervisors WHERE domain = 'chat') WHERE domain = 'chat';
