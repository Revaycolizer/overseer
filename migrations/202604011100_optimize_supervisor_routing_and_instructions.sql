-- Refine Stealth Supervisor Keywords to prevent hijacking basic conversational queries
UPDATE supervisors 
SET fast_path_keywords = 'research, search, browse, latest, documentation, news, how to, guide, tutorial, github, release, update, terminal, command, shell, extract, summarize, evidence, info, explore'
WHERE domain = 'stealth';

-- Update Behavioral Instructions for Chat and Stealth Supervisors to prioritize internal knowledge and suppress evidence
UPDATE supervisors 
SET instructions = instructions || '

═══ KNOWLEDGE & EVIDENCE RULES ═══
- KNOWLEDGE FIRST: If the user asks for static, general knowledge (e.g., historical dates, common facts), answer immediately from your internal knowledge WITHOUT using research tools. Use research tools ONLY for dynamic, recent, or specialized information.
- EVIDENCE SUPPRESSION: Include the "evidence" JSON field ONLY if the user explicitly asked for "sources", "links", "citations", "evidence", or "verification". Otherwise, ALWAYS return an empty array [] for "evidence".'
WHERE domain IN ('chat', 'stealth');
