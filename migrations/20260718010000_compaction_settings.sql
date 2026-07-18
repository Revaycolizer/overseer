-- 20260718010000_compaction_settings.sql
-- V4.17.16: configurable memory compaction model + token threshold.
-- compaction_model controls which LLM is used to summarise older
-- turns. compaction_token_threshold controls the estimated token
-- count at which compaction triggers (model-dependent; 4096 is
-- the safe default for 8K-context models like llama3.2:1b).

INSERT OR IGNORE INTO settings (key, value) VALUES
    ('compaction_model',           '__use_first_available__'),
    ('compaction_token_threshold', '4096');
