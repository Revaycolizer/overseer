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
        _agents: &[crate::engine::utils::discovery::AgentInfo], 
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
        all_agents: Vec<crate::engine::utils::discovery::AgentInfo>,
    ) -> String {
        // 1. Build a dynamic Registry of Capabilities
        let mut registry = String::new();
        registry.push_str("Domain | Purpose | Equipped Tools\n");
        registry.push_str("-------|---------|----------------\n");
        
        for agent in all_agents {
            if agent.domain == "orchestrator" || agent.domain == "overseer" { continue; }
            let tools = agent.tools.join(", ");
            registry.push_str(&format!(
                "{} | {} | [{}]\n",
                agent.domain,
                agent.description,
                if tools.is_empty() { "none" } else { &tools }
            ));
        }

        let coordination_rules = r#"
═══ COORDINATION PROTOCOL ═══
1. INTERNAL APP GUIDANCE: If the query is specifically about USING this Overseer application (how to add agents, tool formats, workshops, or app usage), you MUST route to the 'helper' domain for a single-step chat response. Do NOT plan research or document generation for Overseer usage help.
2. DELEGATION: For all other tasks, assign based on the Domain Purpose and Equipped Tools in the registry.
3. MULTI-STEP PLANNING: If an external task (unrelated to Overseer help) requires research before action (like creating a PDF or Report), you MUST create a 2-step plan.
   Step 1: Use 'stealth' or 'web' to gather evidence.
   Step 2: Use 'office' to generate the document using the evidence from Step 1.
4. CONTEXT PASSING: Explicitly instruct follow-up agents to use findings from specific previous tasks.
5. DOMAIN EXPERTISE: Use 'office' ONLY for file generation. Use 'stealth' for deep research.
"#;

        let final_preamble = self.base_instructions.replace(
            "(THIS SECTION WILL BE DYNAMICALLY INJECTED AT RUNTIME)",
            &format!("{}\n\n{}", registry, coordination_rules)
        );

        let agent = AgentBuilder::new(ollama_client.completion_model(&self.model))
            .preamble(&final_preamble)
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
