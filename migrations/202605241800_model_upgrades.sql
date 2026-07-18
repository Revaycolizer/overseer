-- 202605241800_model_upgrades.sql
-- Migration to configure optimal models via settings table instead of hardcoding.

UPDATE settings SET value = 'llama3.2:1b' WHERE key = 'title_generator_model';
UPDATE settings SET value = 'qwen3:latest' WHERE key = 'planner_model';

INSERT INTO settings (id, key, value)
VALUES (1721827364585, 'vision_model', 'gemma4:latest')
ON CONFLICT(key) DO UPDATE SET value = 'gemma4:latest';

INSERT INTO settings (id, key, value)
VALUES (1721827364586, 'router_model', 'qwen3:latest')
ON CONFLICT(key) DO UPDATE SET value = 'qwen3:latest';
