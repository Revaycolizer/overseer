-- Create Agents table
CREATE TABLE IF NOT EXISTS agents (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    instructions TEXT,
    model TEXT,
    domain TEXT DEFAULT 'desktop',
    version TEXT DEFAULT '1.0.0',
    creator_id BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create Tools table
CREATE TABLE IF NOT EXISTS tools (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    code TEXT,
    language TEXT DEFAULT 'python',
    version TEXT DEFAULT '1.0.0',
    creator_id BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Link Agents and Tools
CREATE TABLE IF NOT EXISTS agent_tools (
    agent_id BIGINT,
    tool_id BIGINT,
    PRIMARY KEY (agent_id, tool_id),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    FOREIGN KEY (tool_id) REFERENCES tools(id) ON DELETE CASCADE
);

-- Conversations
CREATE TABLE IF NOT EXISTS conversations (
    id BIGINT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Messages
CREATE TABLE IF NOT EXISTS messages (
    id BIGINT PRIMARY KEY,
    conversation_id BIGINT NOT NULL,
    role TEXT NOT NULL, -- 'user', 'assistant', 'system'
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);
