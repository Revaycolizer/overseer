-- ═══════════════════════════════════════════════════════════════
-- Update Supervisor Configurations with Fast Path Keywords and Priorities
-- ═══════════════════════════════════════════════════════════════

-- 1. Web Supervisor
UPDATE supervisors SET 
    name = 'Web Supervisor',
    description = 'Automates UI-based web interaction, form filling, and Chrome-specific automation',
    model = 'qwen2.5:7b-instruct',
    fast_path_keywords = 'chrome, browser, automate, website automation, click, type',
    routing_priority = 10
WHERE domain = 'web';

-- 2. Stealth Supervisor
UPDATE supervisors SET 
    name = 'Stealth Supervisor',
    description = 'Orchestrates all web research, search queries, and factual data retrieval',
    model = 'qwen2.5:7b-instruct',
    fast_path_keywords = 'research, search, browse, internet, query, latest, which is, who is',
    routing_priority = 20
WHERE domain = 'stealth';

-- 3. Desktop Supervisor
UPDATE supervisors SET 
    name = 'Desktop Supervisor',
    description = 'Controls native desktop applications and standard OS tasks',
    model = 'qwen2.5:7b-instruct',
    fast_path_keywords = 'app, application, desktop, window, file, directory',
    routing_priority = 15
WHERE domain = 'desktop';

-- 4. Chat Supervisor
UPDATE supervisors SET 
    name = 'Chat Supervisor',
    description = 'General conversation manager and summary assistant',
    model = 'qwen2.5:7b-instruct',
    fast_path_keywords = 'hello, hi, help, explain, what if',
    routing_priority = 5
WHERE domain = 'chat';

-- 5. Computer Supervisor
UPDATE supervisors SET 
    name = 'Computer Supervisor',
    description = 'Broad computer operation with safety boundaries',
    model = 'qwen2.5:7b-instruct',
    fast_path_keywords = 'screen, desktop, computer, operate pc',
    routing_priority = 25
WHERE domain = 'computer';
