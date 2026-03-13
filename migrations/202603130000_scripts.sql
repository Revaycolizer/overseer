-- Add script_path to tools table
ALTER TABLE tools ADD COLUMN script_path TEXT;

-- Update existing tools with default script paths (project-relative)
UPDATE tools SET script_path = 'src/engine/scripts/web/browser_automation.py' WHERE name = 'browser_automation';
UPDATE tools SET script_path = 'src/engine/scripts/desktop/ui_automation.py' WHERE name = 'ui_automation';
UPDATE tools SET script_path = 'src/engine/scripts/stealth/stealth_search.py' WHERE name = 'stealth_search';
UPDATE tools SET script_path = 'src/engine/scripts/stealth/stealth_browse.py' WHERE name = 'stealth_browse';
