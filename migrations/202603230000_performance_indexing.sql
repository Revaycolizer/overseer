-- ═══════════════════════════════════════════════════════════════
-- Performance Indexing for Core Tables
-- ═══════════════════════════════════════════════════════════════

-- 1. Agents and Tools Lookups
CREATE INDEX IF NOT EXISTS idx_agents_name ON agents(name);
CREATE INDEX IF NOT EXISTS idx_agents_domain ON agents(domain);
CREATE INDEX IF NOT EXISTS idx_tools_name ON tools(name);

-- 2. Supervisors and Planners Filtering
CREATE INDEX IF NOT EXISTS idx_supervisors_name ON supervisors(name);
CREATE INDEX IF NOT EXISTS idx_supervisors_domain ON supervisors(domain);
CREATE INDEX IF NOT EXISTS idx_planners_name ON planners(name);
CREATE INDEX IF NOT EXISTS idx_planners_domain ON planners(domain);

-- 3. Junction Tables Foreign Keys
CREATE INDEX IF NOT EXISTS idx_supervisor_agents_ids ON supervisor_agents(supervisor_id, agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_planners_ids ON agent_planners(planner_id, agent_id);

-- 4. Workflow Metadata
CREATE INDEX IF NOT EXISTS idx_workflows_name ON workflows(name);
CREATE INDEX IF NOT EXISTS idx_workflows_status ON workflows(status);

-- Note: settings.key already has a UNIQUE index from its table definition.
