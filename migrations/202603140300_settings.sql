-- Create Settings table
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Seed default settings
INSERT OR IGNORE INTO settings (key, value) VALUES ('title_generator_model', 'qwen2.5:7b-instruct');
INSERT OR IGNORE INTO settings (key, value) VALUES ('default_chat_model', 'llama3:latest');
