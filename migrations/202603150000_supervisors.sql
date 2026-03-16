-- ═══════════════════════════════════════════════════════════════
-- Dynamic Supervisor Hierarchy (Snowflake IDs)
-- ═══════════════════════════════════════════════════════════════

-- 1. Create supervisors table
CREATE TABLE IF NOT EXISTS supervisors (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    instructions TEXT,
    model TEXT,
    domain TEXT UNIQUE NOT NULL, -- e.g. 'web', 'office', 'stealth'
    version TEXT DEFAULT '1.0.0',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 2. Create supervisor_agents junction table for relationships
CREATE TABLE IF NOT EXISTS supervisor_agents (
    id BIGINT PRIMARY KEY,
    supervisor_id BIGINT NOT NULL,
    agent_id BIGINT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(supervisor_id) REFERENCES supervisors(id) ON DELETE CASCADE,
    FOREIGN KEY(agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    UNIQUE(supervisor_id, agent_id)
);
