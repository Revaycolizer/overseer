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
1. INTERNAL APP GUIDANCE: If the query is specifically about USING this Overseer application, you MUST route to the 'helper' domain for a single-step chat response. Do NOT plan research or document generation for Overseer usage help.
2. DELEGATION: For all other tasks, assign domains based on 'Equipped Tools' in the Registry. 
3. MANDATORY MULTI-STEP PLANNING (GATHER -> ACTION): 
   - Rule: Any request requiring information or document generation MUST be a 2-step plan.
   - Step 1 (GATHER): Use a domain equipped with search tools ('stealth', 'web') to collect evidence.
   - Step 2 (ACTION/SYNTHESIS): Use the research to fulfill the user's ultimate goal.
   - DOMAIN SELECTION: If the goal is a FILE (PDF, DOCX, CSV, Report), pick a domain equipped with 'document_writer'. NEVER pick 'chat' for file generation.

═══ PLANNING EXAMPLES ═══
Input: "Generate a PDF about malaria"
Output: [
  {"domain": "stealth", "task": "Research comprehensive information about malaria causes, symptoms, and treatments."},
  {"domain": "office", "task": "Use the findings from Task #1 to generate a professional PDF report about malaria."}
]

Input: "Research AI agent reliability"
Output: [
  {"domain": "stealth", "task": "Research current best practices and metrics for AI agent reliability."},
  {"domain": "chat", "task": "Based on Task #1, provide a professional summary of AI agent reliability practices to the user."}
]

4. CONTEXT SHARING: You MUST explicitly instruct follow-up agents to consume and respect findings from specific previous tasks (e.g., "Use findings from Task #1 to answer").
5. DOMAIN EXPERTISE: Use specialized agents ONLY for their equipped tools.
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
