use rig::{
    agent::AgentBuilder,
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};

pub struct NeontunePlanner {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl NeontunePlanner {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let preamble = r#"You are a Neontune Domain Planner.
YOUR JOB:
Break down a high-level Neontune music player macro-task into specific, ordered, atomic steps.
Return ONLY a valid JSON array of strings.

═══ NEONTUNE DECOMPOSITION RULES ═══
1. Steps MUST be simple and single-action (e.g., 'Launch Neontune', 'Search for Tali Rush - U', 'Click on the first search result').
2. Steps MUST be self-contained and specify 'Neontune' in EVERY step.
3. Never combine multiple actions in one step.
4. Ensure logical flow (e.g., launch before search, search before clicking results).

═══ VERBATIM RULE (CRITICAL) ═══
- For song titles or search queries, you MUST use the EXACT characters provided by the user.

═══ EXAMPLES ═══
Input: 'play Tali Rush - U in Neontune'
Output: ["Launch Neontune", "Search for 'Tali Rush - U' in Neontune", "Click on the 'Tali Rush - U' result to play"]

═══ FORMAT ENFORCEMENT ═══
- Output MUST be a valid JSON array of STRINGS.
- Format: ["Step 1", "Step 2"]
- NO MARKDOWN. NO CODE BLOCKS. NO EXPLANATIONS. JUST THE JSON ARRAY.
"#;

        let agent = AgentBuilder::new(ollama_client.completion_model("llama3:latest"))
            .preamble(preamble)
            .default_max_turns(1)
            .build();
        Self { agent }
    }

    pub async fn plan(&self, prompt: &str) -> String {
        self.agent
            .prompt(prompt)
            .await
            .unwrap_or_else(|e| format!("[\"Error planning Neontune task: {}\"]", e))
    }
}
