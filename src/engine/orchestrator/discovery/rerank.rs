// src/engine/orchestrator/discovery/rerank.rs
use crate::engine::utils::discovery::AgentInfo;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::ollama;

pub async fn rerank_agents(
    query: &str,
    mut agents: Vec<AgentInfo>,
    limit: usize,
    ollama_client: &ollama::Client,
) -> Vec<AgentInfo> {
    if agents.len() <= limit {
        return agents;
    }

    let mut candidates = String::new();
    for (i, agent) in agents.iter().enumerate() {
        candidates.push_str(&format!(
            "{}. [{}] {}\n",
            i, agent.domain, agent.description
        ));
    }

    let rerank_prompt = format!(
        "USER_QUERY: {}\n\nCANDIDATE_AGENTS:\n{}\n\nINSTRUCTION: Select Top {} agents. Respond with ONLY indices (0, 2, 5).",
        query, candidates, limit
    );

    let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model("llama3.2:1b"))
        .preamble("You are a specialized agent selector.")
        .build();

    if let Ok(resp) = agent.prompt(&rerank_prompt).await {
        let indices: Vec<usize> = resp
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .filter(|i| *i < agents.len())
            .collect();

        if !indices.is_empty() {
            let mut top = Vec::new();
            for idx in indices.iter().take(limit) {
                top.push(agents[*idx].clone());
            }

            // Fallback for core domains
            for core in vec!["stealth", "office", "web", "chat"] {
                if top.len() < limit && !top.iter().any(|a| a.domain == core) {
                    if let Some(a) = agents.iter().find(|a| a.domain == core) {
                        top.push(a.clone());
                    }
                }
            }
            return top;
        }
    }
    agents.truncate(limit);
    agents
}
