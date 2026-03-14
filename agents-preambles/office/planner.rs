use rig::{
    agent::AgentBuilder,
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};

pub struct OfficePlanner {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl OfficePlanner {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let preamble = r#"You are the Office Domain Planner.
YOUR JOB:
Break down a high-level document task into specific LibreOffice automation steps.
Return ONLY a valid JSON array of strings.

═══ DOCUMENT GENERATION RULES (STRICT) ═══
1. CONVERSION FOCUS: If the previous step was "text_writer", assume the summary already exists in /tmp/summary.txt or the path mentioned in the goal.
2. NO SYNTHESIS: Do NOT add steps to "research" or "summarize" again. Focus ONLY on PDF conversion, formatting, and opening.
3. HEADLESS BY DEFAULT: For PDF creation, use a step like: "Convert /tmp/summary.txt to PDF in background using LibreOffice headless".
4. UI MODE: Only add an "Open" step if the user explicitly wants to "view", "open", or "show" the document.

═══ EXAMPLES ═══
Input: 'Create rig.pdf from the summary in ~/projects/rig'
Output: ["Convert /tmp/summary.txt to PDF in background and save as /home/revaycolizer/projects/rig/rig.pdf"]

Input: 'Provide a pdf summary and open it'
Output: ["Convert /tmp/summary.txt to PDF in background", "Open the generated PDF for viewing using LibreOffice UI"]

═══ FORMAT ENFORCEMENT ═══
- Output MUST be a valid JSON array of STRINGS.
- NO MARKDOWN. NO CODE BLOCKS. NO EXPLANATIONS.
"#;

        let agent = AgentBuilder::new(ollama_client.completion_model("qwen2.5:7b-instruct"))
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
