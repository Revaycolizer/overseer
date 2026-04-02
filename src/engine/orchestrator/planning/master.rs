use crate::engine::utils::discovery::AgentInfo;
use rig::{agent::AgentBuilder, client::CompletionClient, completion::Prompt, providers::ollama};

#[derive(Clone)]
pub struct MasterPlanner {
    pub model: String,
    pub base_instructions: String,
}

impl MasterPlanner {
    pub fn new(
        _ollama_client: &ollama::Client,
        model: &str,
        base_instructions: &str,
        _agents: &[AgentInfo], // Legacy, will be ignored
    ) -> Self {
        Self {
            model: model.to_string(),
            base_instructions: base_instructions.to_string(),
        }
    }

    pub async fn plan(
        &self,
        ollama_client: &ollama::Client,
        prompt: &str,
        agents: Vec<crate::db::Supervisor>,
    ) -> String {
        let mut domains_str = String::new();
        for (i, agent) in agents.iter().enumerate() {
            domains_str.push_str(&format!(
                "{}. \"{}\" - {}\n",
                i + 1,
                agent.domain,
                agent.description.as_deref().unwrap_or("No description")
            ));
        }

        let coordination_rules = r#"
═══ COORDINATION RULES ═══
1. MULTI-STEP PLANNING: If a task requires information gathering (e.g. "Research [Topic]") before an action (e.g. "Generate a [Format]"), you MUST create a 2-step plan.
2. STEP 1: Research/Data Gathering (usually 'stealth' or 'chat' domain).
3. STEP 2: Execution/Output (e.g. 'office' for documents, 'computer' for UI, 'chat' for summaries).
4. DATA FLOW: Explicitly state in the second task that it should use the specific results and entities discovered in the first step.
"#;

        let preamble = format!(
            r#"{}\n\n{}\n\n═══ AVAILABLE DOMAINS ═══\n{}\n"#,
            self.base_instructions, coordination_rules, domains_str
        );

        let agent = AgentBuilder::new(ollama_client.completion_model(&self.model))
            .preamble(&preamble)
            .default_max_turns(1)
            .build();

        let safe_prompt = if prompt.trim().is_empty() {
            "current user query"
        } else {
            prompt
        };

        agent
            .prompt(safe_prompt)
            .await
            .unwrap_or_else(|e| format!("[{{\"domain\": \"error\", \"task\": \"{}\"}}]", e))
    }
}
