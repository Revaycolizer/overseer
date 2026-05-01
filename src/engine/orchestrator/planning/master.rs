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
1. INTERNAL APP GUIDANCE: If the query is specifically about USING this Overseer application, you MUST route to the 'helper' domain for a single-step chat response.

2. DOMAIN DISCOVERY: Use the 'Equipped Tools' column in the Registry. NEVER assign a task to a domain that doesn't have the required tool.

3. MANDATORY MULTI-STEP WORKFLOW (THINK THEN ACT): 
   - Any request involving information gathering, research, or cross-domain execution MUST have at least TWO steps.
   - STEP 1 (GATHER): Always start with a research domain ('stealth' or 'web') to collect facts.
   - STEP 2 (ACTION/DELIVERY): Follow up with a synthesis domain ('office', 'text_writer', or 'chat') to deliver results.

4. DOCUMENT GENERATION RULES: 
   - If the user asks for a FILE (PDF, Word, DOCX, CSV, XLSX, Report, Chart), you MUST route the final step to a domain with 'document_writer' or 'charting'.
   - NEVER pick 'chat' to generate or deliver a structured file. Use 'chat' ONLY for conversational summaries.
   - ARTIFACT MANDATE: A plan for an artifact MUST ALWAYS have a Research/Search step first. No exceptions.

5. CRITICAL CONSTRAINTS:
   - NO SINGLE-STEP PLANS: If the query contains "research", "find", "generate", "pdf", or "report", a 1-step plan is STRICTLY FORBIDDEN and will be REJECTED.
   - ZERO-KNOWLEDGE: You must assume the system knows NOTHING about the topic initially. Always start with a search.
   - EXPLICIT DATA LINKING: In Step 2, you MUST explicitly list the findings from Step 1 that should be included (e.g. "Include the discovered causes and symptoms in the PDF").

═══ PLANNING FAILURES (DO NOT REPEAT) ═══
❌ BAD PLAN (Collapsed): [{"domain": "chat", "task": "Research and tell me about malaria"}]
✅ GOOD PLAN (Decomposed): [
  {"domain": "stealth", "task": "Research malaria..."},
  {"domain": "chat", "task": "Summarize the research..."}
]

❌ BAD PLAN (Phantom Tool): [{"domain": "chat", "task": "Generate a PDF summary of X"}] (Chat has no PDF tool)
✅ GOOD PLAN (Correct Domain): [
  {"domain": "stealth", "task": "Get details on X..."},
  {"domain": "office", "task": "Generate a PDF report on X using findings..."}
]

❌ BAD PLAN (Generative Procrastination): [{"domain": "office", "task": "Generate a PDF about X"}] (Missing research step)
✅ GOOD PLAN (Silicon Mandate): [
  {"domain": "stealth", "task": "Conduct thorough research on X..."},
  {"domain": "office", "task": "Using the findings from Task #1, generate the final PDF..."}
]

═══ PLANNING EXAMPLES ═══
Input: "Generate a PDF about malaria"
Output: [
  {"domain": "stealth", "task": "Research comprehensive information about malaria causes, symptoms, and treatments. Provide all findings in the research results."},
  {"domain": "office", "task": "Using the research from Task #1, generate a professional malaria report in PDF format. Ensure the sections for causes, symptoms, and treatments are fully populated with the details found."}
]

Input: "How is the stock market doing?"
Output: [
  {"domain": "stealth", "task": "Search for current stock market indices (S&P 500, DOW, NASDAQ) and today's top market news."},
  {"domain": "chat", "task": "Based on the market data from Task #1 (indices and top news), provide a concise summary of today's stock performance to the user."}
]
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
