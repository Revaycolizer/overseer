-- Standalone supervisor seeding migration.
-- The full_data_seed.sql migration fails silently on tool INSERTs (embedded scripts
-- break the SQL parser), which prevents supervisor INSERTs from running.
-- This migration ensures supervisors always exist regardless of tool seeding.

INSERT OR IGNORE INTO supervisors (id, name, description, instructions, model, domain, version, created_at, updated_at, bypass_synthesizer)
VALUES (7438818720128045056, 'Web Supervisor', 'Automates UI-based web interaction, form filling, and Chrome-specific automation',
'You are the Web Supervisor supervisor. Your mission is to complete the user request using the available tools. IMAGE RULE: If search results contain images, you ARE authorized to show them directly in your summary via markdown ![](). Never say "I cannot show images" to the user.',
'qwen2.5:7b-instruct', 'web', '1.0.0', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0);

INSERT OR IGNORE INTO supervisors (id, name, description, instructions, model, domain, version, created_at, updated_at, bypass_synthesizer)
VALUES (7438818720132239361, 'Desktop Supervisor', 'Controls native desktop applications and standard OS tasks',
'You are the Desktop Supervisor supervisor. Your mission is to complete the user request using the available tools. IMAGE RULE: If search results contain images, you ARE authorized to show them directly in your summary via markdown ![](). Never say "I cannot show images" to the user.',
'qwen2.5:7b-instruct', 'desktop', '1.0.0', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0);

INSERT OR IGNORE INTO supervisors (id, name, description, instructions, model, domain, version, created_at, updated_at, bypass_synthesizer)
VALUES (7438818720144822272, 'Stealth Supervisor', 'Orchestrates all web research, search queries, and factual data retrieval',
'You are the Stealth Supervisor supervisor. Your mission is to complete the user request using the available tools. IMAGE RULE: If search results contain images, you ARE authorized to show them directly in your summary via markdown ![](). Never say "I cannot show images" to the user.',
'qwen2.5:7b-instruct', 'stealth', '1.0.0', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0);

INSERT OR IGNORE INTO supervisors (id, name, description, instructions, model, domain, version, created_at, updated_at, bypass_synthesizer)
VALUES (7438818720132239360, 'Office Supervisor', 'Executes LibreOffice document operations',
'You are the Office Supervisor supervisor. Your mission is to complete the user request using the available tools.',
'qwen2.5:7b-instruct', 'office', '1.0.0', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0);

INSERT OR IGNORE INTO supervisors (id, name, description, instructions, model, domain, version, created_at, updated_at, bypass_synthesizer)
VALUES (7438818720132239363, 'Neontune Supervisor', 'Music and audio management assistant',
'You are the Neontune Supervisor supervisor. Your mission is to complete the user request using the available tools.',
'qwen2.5:7b-instruct', 'neontune', '1.0.0', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0);

INSERT OR IGNORE INTO supervisors (id, name, description, instructions, model, domain, version, created_at, updated_at, bypass_synthesizer)
VALUES (7438818720144822273, 'Chat Supervisor', 'General conversation manager and summary assistant',
'You are the CHAT SUPERVISOR. You answer conversational questions directly without tools.',
'qwen2.5:7b-instruct', 'chat', '1.0.0', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0);

INSERT OR IGNORE INTO supervisors (id, name, description, instructions, model, domain, version, created_at, updated_at, bypass_synthesizer)
VALUES (7438818720144822274, 'Computer Supervisor', 'Broad computer operation with safety boundaries',
'You are the Computer Supervisor supervisor. Your mission is to complete the user request using the available tools. IMAGE RULE: If search results contain images, you ARE authorized to show them directly in your summary via markdown ![](). Never say "I cannot show images" to the user.',
'qwen2.5:7b-instruct', 'computer', '1.0.0', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0);

-- Also seed settings that may have been missed
INSERT OR IGNORE INTO settings (id, key, value) VALUES (abs(random()), 'artifact_routing_hint', '[ARTIFACTS ATTACHED] The user has selected local file artifacts as context for this request. Route to ''chat'' or ''vision'' domain so the agent can answer from the attached content. Do NOT route to ''stealth'' (search) unless the user explicitly asks to search the web.');
INSERT OR IGNORE INTO settings (id, key, value) VALUES (abs(random()), 'artifact_context_header', 'ATTACHED ARTIFACTS (the user explicitly selected these as context — use them to answer):');
INSERT OR IGNORE INTO settings (id, key, value) VALUES (abs(random()), 'artifact_default_domain', 'chat');
INSERT OR IGNORE INTO settings (id, key, value) VALUES (abs(random()), 'artifact_media_domain', 'chat');

-- Apply fast-path keywords and routing config to supervisors
UPDATE supervisors SET fast_path_keywords = 'chrome, browser, automate, website automation, click, type, website, internet, web', routing_priority = 10 WHERE domain = 'web' AND fast_path_keywords IS NULL;
UPDATE supervisors SET fast_path_keywords = 'research, search, browse, internet, query, latest, which is, who is', routing_priority = 20 WHERE domain = 'stealth' AND fast_path_keywords IS NULL;
UPDATE supervisors SET fast_path_keywords = 'app, application, desktop, window, file, directory, find file, locate file, resume', routing_priority = 15 WHERE domain = 'desktop' AND fast_path_keywords IS NULL;
UPDATE supervisors SET fast_path_keywords = 'hello, hi, help, explain, what if, who are you, tell me about', routing_priority = 5, skip_fast_path_if_artifacts = 1, artifact_fast_path_keywords = 'explain,summarize,analyze,read,describe,what is this,tell me about,review,interpret,break down,walk me through' WHERE domain = 'chat' AND fast_path_keywords IS NULL;
UPDATE supervisors SET fast_path_keywords = 'screen, desktop, computer, operate pc, screenshot, capture, observe, inspect, watch, background, running, process, task, currently, list, show, ocr, analyze, vision, image, picture, photo, summarize', routing_priority = 25 WHERE domain = 'computer' AND fast_path_keywords IS NULL;
UPDATE supervisors SET fast_path_keywords = 'music, play music, play song, audio, song, neontune, volume, pause, next', routing_priority = 5 WHERE domain = 'neontune' AND fast_path_keywords IS NULL;
