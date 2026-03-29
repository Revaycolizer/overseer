-- Robust final seeding for Overseer Artifact functionality.
-- Fixes missing supervisors, missing supervisor links, and missing settings.

PRAGMA foreign_keys = OFF;

-- Ensure all necessary supervisors exist.
-- Including the 'computer' supervisor which is required for media artifact routing.
INSERT OR REPLACE INTO supervisors (id, name, description, instructions, model, domain, version, bypass_synthesizer)
VALUES (7438818720128045056, 'Web Supervisor', 'Automates UI-based web interaction', 'You are the Web Supervisor.', 'qwen2.5:7b-instruct', 'web', '1.0.0', 0);

INSERT OR REPLACE INTO supervisors (id, name, description, instructions, model, domain, version, bypass_synthesizer)
VALUES (7438818720144822272, 'Stealth Supervisor', 'Methodology-First Research Specialist', 'You are the Stealth Supervisor.', 'qwen2.5:7b-instruct', 'stealth', '1.0.0', 0);

INSERT OR REPLACE INTO supervisors (id, name, description, instructions, model, domain, version, bypass_synthesizer)
VALUES (7438818720132239361, 'Desktop Supervisor', 'Controls native desktop applications', 'You are the Desktop Supervisor.', 'qwen2.5:7b-instruct', 'desktop', '1.0.0', 0);

INSERT OR REPLACE INTO supervisors (id, name, description, instructions, model, domain, version, bypass_synthesizer)
VALUES (7438818720144822273, 'Chat Supervisor', 'General conversation and artifact analysis', 'You are the Chat Supervisor.', 'qwen2.5:7b-instruct', 'chat', '1.0.0', 0);

INSERT OR REPLACE INTO supervisors (id, name, description, instructions, model, domain, version, bypass_synthesizer)
VALUES (7438818720132239360, 'Office Supervisor', 'Document management assistant', 'You are the Office Supervisor.', 'qwen2.5:7b-instruct', 'office', '1.0.0', 0);

INSERT OR REPLACE INTO supervisors (id, name, description, instructions, model, domain, version, bypass_synthesizer)
VALUES (7438818720132239363, 'Neontune Supervisor', 'Music automation specialist', 'You are the Neontune Supervisor.', 'qwen2.5:7b-instruct', 'neontune', '1.0.0', 0);

INSERT OR REPLACE INTO supervisors (id, name, description, instructions, model, domain, version, bypass_synthesizer)
VALUES (7438818720144822274, 'Computer Supervisor', 'Broad computer operation and vision analysis', 'You are the Computer Supervisor. Use vision tools for images.', 'qwen2.5:7b-instruct', 'computer', '1.0.0', 0);

-- Clear old supervisor-agent links to avoid duplicates or broken mappings.
DELETE FROM supervisor_agents WHERE supervisor_id IN (
    7438818720128045056, -- web
    7438818720144822272, -- stealth
    7438818720132239361, -- desktop
    7438818720144822273, -- chat
    7438818720132239360, -- office
    7438818720132239363, -- neontune
    7438818720144822274  -- computer
);

-- Link supervisors to their primary agents.
-- Use abs(random()) for junction IDs to avoid conflicts.
INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (abs(random()), 7438818720128045056, 7438474706661019649); -- web -> Web Agent
INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (abs(random()), 7438818720144822272, 7438474706661019648); -- stealth -> Stealth Agent
INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (abs(random()), 7438818720132239361, 7438649930752856068); -- desktop -> Desktop Supervisor Agent
INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (abs(random()), 7438818720144822273, 7438474706665213954); -- chat -> Chat Assistant
INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (abs(random()), 7438818720132239360, 7438649930757050370); -- office -> Text Writer
INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (abs(random()), 7438818720132239363, 7438900000000000001); -- neontune -> Neontune Agent
INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (abs(random()), 7438818720144822274, 7438474706665213955); -- computer -> Vision Assistant

-- Also link Computer Supervisor to Stealth Agent for research capabilities.
INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (abs(random()), 7438818720144822274, 7438474706661019648);

-- Push configurations to these supervisors.
UPDATE supervisors SET fast_path_keywords = 'chrome, browser, automate, website automation, click, type, website, web, dashboard, form, fill, login, interactive', routing_priority = 10 WHERE domain = 'web';
UPDATE supervisors SET fast_path_keywords = 'research, search, browse, query, latest, which is, who is, check, version, documentation, news, how to, guide, tutorial, github, release, update, terminal, command, shell', routing_priority = 20 WHERE domain = 'stealth';
UPDATE supervisors SET fast_path_keywords = 'app, application, desktop, window, file, directory, find file, locate file, resume', routing_priority = 15 WHERE domain = 'desktop';
UPDATE supervisors SET fast_path_keywords = 'hello, hi, help, explain, what if, who are you, tell me about', routing_priority = 5, skip_fast_path_if_artifacts = 1, artifact_fast_path_keywords = 'explain,summarize,analyze,read,describe,what is this,tell me about,review,interpret,break down,walk me through' WHERE domain = 'chat';
UPDATE supervisors SET fast_path_keywords = 'screen, desktop, computer, operate pc, screenshot, capture, observe, inspect, watch, background, running, process, task, currently, list, show, ocr, analyze, vision, image, picture, photo, summarize', routing_priority = 25 WHERE domain = 'computer';
UPDATE supervisors SET fast_path_keywords = 'music, play music, play song, audio, song, neontune, volume, pause, next', routing_priority = 5 WHERE domain = 'neontune';

PRAGMA foreign_keys = ON;
