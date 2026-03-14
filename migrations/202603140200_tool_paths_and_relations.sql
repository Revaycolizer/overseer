-- Fix tool script_paths to point to agents-scripts/ directory
-- and create agent_tools many-to-many relationships

-- ═══════════════════════════════════════════════════════════════
-- 1. Fix script_path for all tools to point to agents-scripts/
-- ═══════════════════════════════════════════════════════════════

UPDATE tools SET script_path = 'agents-scripts/web/browser_automation.py' WHERE name = 'browser_automation';
UPDATE tools SET script_path = 'agents-scripts/web/browser_server.py' WHERE name = 'browser_server';
UPDATE tools SET script_path = 'agents-scripts/stealth/stealth_search.py' WHERE name = 'stealth_search';
UPDATE tools SET script_path = 'agents-scripts/stealth/stealth_browse.py' WHERE name = 'stealth_browse';
UPDATE tools SET script_path = 'agents-scripts/desktop/ui_automation.py' WHERE name = 'ui_automation';
UPDATE tools SET script_path = 'agents-scripts/desktop/calibrate.py' WHERE name = 'calibrate';
UPDATE tools SET script_path = 'agents-scripts/desktop/configure_app.py' WHERE name = 'configure_app';
UPDATE tools SET script_path = 'agents-scripts/desktop/diagnose_ui.py' WHERE name = 'diagnose_ui';
UPDATE tools SET script_path = 'agents-scripts/desktop/find_search_field.py' WHERE name = 'find_search_field';
UPDATE tools SET script_path = 'agents-scripts/desktop/get_mouse_position.py' WHERE name = 'get_mouse_position';
UPDATE tools SET script_path = 'agents-scripts/desktop/ocr_coordinates.py' WHERE name = 'ocr_coordinates';
UPDATE tools SET script_path = 'agents-scripts/desktop/ocr_dump.py' WHERE name = 'ocr_dump';
UPDATE tools SET script_path = 'agents-scripts/desktop/paddle_ui_automation.py' WHERE name = 'paddle_ui_automation';
UPDATE tools SET script_path = 'agents-scripts/desktop/verify_layout.py' WHERE name = 'verify_layout';
UPDATE tools SET script_path = 'agents-scripts/desktop/verify_window_capture.py' WHERE name = 'verify_window_capture';
UPDATE tools SET script_path = 'agents-scripts/desktop/vision_debug.py' WHERE name = 'vision_debug';

-- Also update the tool code from files on disk
-- (This is handled at runtime by the backend when tools are saved)

-- ═══════════════════════════════════════════════════════════════
-- 2. Create agent_tools many-to-many relationships
-- ═══════════════════════════════════════════════════════════════

-- orchestrator ↔ browser_automation, ui_automation, os_tool
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'orchestrator' AND t.name = 'browser_automation';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'orchestrator' AND t.name = 'ui_automation';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'orchestrator' AND t.name = 'os_tool';

-- web_planner ↔ browser_automation
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'web_planner' AND t.name = 'browser_automation';

-- web_supervisor ↔ browser_automation, browser_server
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'web_supervisor' AND t.name = 'browser_automation';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'web_supervisor' AND t.name = 'browser_server';

-- stealth ↔ stealth_search, stealth_browse
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'stealth' AND t.name = 'stealth_search';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'stealth' AND t.name = 'stealth_browse';

-- desktop_planner ↔ ui_automation
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_planner' AND t.name = 'ui_automation';

-- desktop_supervisor ↔ ui_automation, calibrate, diagnose_ui, ocr_coordinates, ocr_dump, vision_debug
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'ui_automation';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'calibrate';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'diagnose_ui';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'ocr_coordinates';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'ocr_dump';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'vision_debug';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'configure_app';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'find_search_field';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'verify_layout';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'verify_window_capture';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'get_mouse_position';
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'desktop_supervisor' AND t.name = 'paddle_ui_automation';

-- neontune_planner ↔ ui_automation
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'neontune_planner' AND t.name = 'ui_automation';

-- neontune_supervisor ↔ ui_automation
INSERT OR IGNORE INTO agent_tools (agent_id, tool_id)
SELECT a.id, t.id FROM agents a, tools t WHERE a.domain = 'neontune_supervisor' AND t.name = 'ui_automation';
