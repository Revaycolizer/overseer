-- Refine supervisor keywords and flags for better artifact routing
-- 100% database-driven orchestration (v11)

-- 1. Update Stealth Supervisor: Ensure skip_fast_path_if_artifacts is enabled
UPDATE supervisors 
SET skip_fast_path_if_artifacts = 1,
    fast_path_keywords = 'research, search, browse, internet, query, latest, which is, who is'
WHERE domain = 'stealth';

-- 2. Update Web Supervisor: Ensure skip_fast_path_if_artifacts is enabled and expand keywords
UPDATE supervisors
SET skip_fast_path_if_artifacts = 1,
    fast_path_keywords = 'chrome, browser, automate, website automation, click, type, website, internet, web'
WHERE domain = 'web';

-- 3. Update Computer Supervisor: Expand keywords to include vision, OCR and system observation patterns
UPDATE supervisors
SET fast_path_keywords = 'screen, desktop, computer, operate pc, screenshot, capture, observe, inspect, watch, background, running, process, task, currently, list, show, ocr, analyze, vision, image, picture, photo, summarize, paddleocr, paddle'
WHERE domain = 'computer';

-- 4. Update Desktop Supervisor: Expand keywords for file and app patterns
UPDATE supervisors
SET fast_path_keywords = 'app, application, desktop, window, file, directory, find file, locate file, resume'
WHERE domain = 'desktop';

-- 5. Update Chat Supervisor: Refine keywords
UPDATE supervisors
SET fast_path_keywords = 'hello, hi, help, explain, what if, who are you, tell me about'
WHERE domain = 'chat';

-- 6. Update Neontune Supervisor: Add music and audio keywords
UPDATE supervisors
SET fast_path_keywords = 'music, play music, play song, audio, song, neontune, volume, pause, next, music theory, chords, lyrics'
WHERE domain = 'neontune';
