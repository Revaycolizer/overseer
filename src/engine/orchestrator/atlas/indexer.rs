use crate::engine::utils::discovery::AgentInfo;

pub struct AgentAtlas {
    pub agents: Vec<AgentInfo>,
}

impl AgentAtlas {
    pub fn new(agents: Vec<AgentInfo>) -> Self {
        Self { agents }
    }

    pub fn get_by_domain(&self, domain: &str) -> Option<&AgentInfo> {
        self.agents.iter().find(|a| a.domain == domain)
    }

    pub fn get_all_domains(&self) -> Vec<String> {
        self.agents.iter().map(|a| a.domain.clone()).collect()
    }
}

pub fn vectorize_agent(agent: &AgentInfo) -> String {
    format!("{}: {}", agent.domain, agent.description)
}
