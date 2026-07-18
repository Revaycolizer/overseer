-- Seed default lock screen timeout (30 minutes in seconds = 1800)
INSERT OR IGNORE INTO settings (id, key, value) VALUES (1721827364582, 'idle_lock_timeout_secs', '1800');
