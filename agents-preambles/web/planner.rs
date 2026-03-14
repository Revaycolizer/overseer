use rig::{
    agent::AgentBuilder,
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};
use crate::skills;

pub struct WebPlanner {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl WebPlanner {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let browser_skill = skills::BROWSER_SKILL.content;

        let preamble = format!(
                r#"You are a Web Domain Planner.
YOUR JOB:
Break down a high-level web macro-task into specific, ordered, atomic browser automation steps.
Return ONLY a valid JSON array of strings.

═══ WEB DECOMPOSITION RULES ═══
1. Steps MUST be simple and single-action (e.g., 'Open Chrome', 'Search for X', 'Click on Y').
2. Steps MUST be self-contained and specify the target application in EVERY step (e.g., 'Click on result in Chrome').
3. Never combine multiple actions in one step.
4. Ensure logical flow (e.g., search before clicking a result).

═══ VERBATIM RULE (CRITICAL) ═══
- For search queries or specific text inputs, you MUST use the EXACT characters provided by the user.
- NEVER truncate query suffixes!

═══ NAVIGATION PRINCIPLES ═══
- If a user says 'open X', use 'Open X'.
- If a user says 'click on link', use 'Click on link in [App]'.

{}

═══ EXAMPLES ═══
Input: 'Go to Revaycolizer github and find specific repo details'
Output: ["Search for 'Revaycolizer github' in Chrome", "Click on the GitHub profile link", "Click on the target repository link"]

═══ FORMAT ENFORCEMENT ═══
- Output MUST be a valid JSON array of STRINGS.
- Format: ["Step 1", "Step 2"]
- NO MARKDOWN. NO CODE BLOCKS. NO EXPLANATIONS. JUST THE JSON ARRAY.
"#,
                browser_skill
            );

        let agent = AgentBuilder::new(ollama_client.completion_model("llama3:latest"))
            .preamble(&preamble)
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
