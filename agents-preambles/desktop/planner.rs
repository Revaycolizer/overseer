use rig::{
    agent::AgentBuilder,
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};

pub struct DesktopPlanner {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl DesktopPlanner {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let preamble = r#"You are a Desktop App Domain Planner.
YOUR JOB:
Break down a high-level desktop macro-task into specific, ordered, atomic UI automation steps.
Return ONLY a valid JSON array of strings.

═══ DESKTOP DECOMPOSITION RULES ═══
1. Steps MUST be simple and single-action (e.g., 'Open Neontune', 'Search for X', 'Click on Y').
2. Steps MUST be self-contained and specify the target application in EVERY step (e.g., 'Search for jazz in Neontune').
3. Never combine multiple actions in one step.

═══ VERBATIM RULE (CRITICAL) ═══
- For search queries, song names, or specific text inputs, you MUST use the EXACT characters provided by the user.
- NEVER truncate query suffixes! (e.g. 'Tali Rush - U' must remain 'Tali Rush - U', NOT 'Tali Rush').

═══ EXAMPLES ═══
Input: 'Play some jazz in Neontune.'
Output: ["Open Neontune app", "Search for 'jazz music' in Neontune", "Click on the result containing 'Jazz'"]

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
            .unwrap_or_else(|e| format!("[\"Error planning: {}\"]", e))
    }
}
