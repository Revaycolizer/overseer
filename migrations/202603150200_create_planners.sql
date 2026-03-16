-- Create planners table
CREATE TABLE IF NOT EXISTS planners (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    instructions TEXT,
    model TEXT,
    domain TEXT NOT NULL,
    version TEXT,
    code TEXT
);

-- Separate mapping table for agents managed by planners (if needed, though supervisors usually manage agents)
-- For now, planners are domain-specific hierarchical components.
CREATE TABLE IF NOT EXISTS agent_planners (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    planner_id INTEGER NOT NULL,
    agent_id INTEGER NOT NULL,
    FOREIGN KEY(planner_id) REFERENCES planners(id),
    FOREIGN KEY(agent_id) REFERENCES agents(id)
);

-- Migrate existing planners from agents table to planners table
INSERT INTO planners (id, name, description, instructions, model, domain, version, code)
SELECT id, name, description, instructions, model, domain, version, code
FROM agents 
WHERE domain LIKE '%_planner%' OR name LIKE '%Planner%';

-- Remove them from agents table to keep it clean (they are now Planners)
DELETE FROM agents 
WHERE domain LIKE '%_planner%' OR name LIKE '%Planner%';
