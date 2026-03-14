use rig::{
    agent::AgentBuilder,
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};
use crate::engine::utils::discovery::AgentInfo;

#[derive(Clone)]
pub struct MasterPlanner {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl MasterPlanner {
    pub fn new(ollama_client: &ollama::Client, base_instructions: &str, agents: &[AgentInfo]) -> Self {
        let mut domains_str = String::new();
        for (i, agent) in agents.iter().enumerate() {
            // Skip orchestrator and overseer from the planning domain list
            if agent.domain == "orchestrator" || agent.domain == "overseer" { continue; }
            domains_str.push_str(&format!("{}. \"{}\" - {}\n", i + 1, agent.domain, agent.description));
        }

        let preamble = format!(r#"{}\n\n═══ AVAILABLE DOMAINS ═══\n{}\n"#, base_instructions, domains_str);

        let agent = AgentBuilder::new(ollama_client.completion_model("qwen2.5:7b-instruct"))
            .preamble(&preamble)
            .default_max_turns(1)
            .build();
        Self { agent }
    }

    pub async fn plan(&self, prompt: &str) -> String {
        self.agent
            .prompt(prompt)
            .await
            .unwrap_or_else(|e| format!("[{{\"domain\": \"error\", \"task\": \"{}\"}}]", e))
    }
}
