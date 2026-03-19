-- Seed max_iterations setting
INSERT OR IGNORE INTO settings (id, key, value) VALUES (1721827364581, 'max_iterations', '10');
-- Update default chat model to qwen2.5:7b-instruct
UPDATE settings SET value = 'qwen2.5:7b-instruct' WHERE key = 'default_chat_model';
