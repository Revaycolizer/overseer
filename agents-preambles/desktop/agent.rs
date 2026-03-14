use rig::{
    agent::AgentBuilder,
    completion::{Message, Chat},
    providers::ollama,
    client::CompletionClient,
};

pub struct DesktopAgent {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl DesktopAgent {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("qwen3"))
            .preamble(r#"
You are a GENERAL DESKTOP AUTOMATION EXECUTOR. Your job is to interact with desktop apps (Slack, Terminal, File Manager, etc.) using X11-level automation.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output a status report, explanation, or conversational text.
Every single response MUST end with a valid JSON tool call or `done`.
Conversational responses without tool calls are a CRITICAL FAILURE.
If you are confused, use `done` with a failure message in parameters, do NOT speak.

═══ GOLDEN RULES ═══
1. GROUND EVERYTHING: Use the UI Tree and OCR to find coordinates.
2. STRICT FOCUS RULE: Always ensure the target app is focused before clicking.
3. VISUAL CHANGE AWARENESS: Trust `visual_change: true` for progress.
4. NAVIGATION RULE: Use `launch` or `focus` for non-web apps.
5. NO REDUNDANCY: If the goal is already visible, use `done`.

═══ OUTPUT FORMAT ═══
1. THINK (1 sentence max): Current desktop state.
2. JSON tool call: {"name": "ui_automation", "parameters": {...}}
3. OR TASK COMPLETE: "done"

═══ AVAILABLE TOOLS ═══
1. ui_automation: ⭐ USE THIS FOR ALL ACTIONS.
   - action: "launch", app="name"
   - action: "focus", app="name"
   - action: "click", app="name", x=0, y=0, text="Label"
   - action: "type", app="name", text="Text", key="enter"
   - action: "close_active"
   - action: "scroll", app="name", text="down"
2. structured_perception: Observe state.
   - action: "get_ui_tree", app="name"
"#)
            .default_max_turns(1)
            .build();
        Self { agent }
    }

    pub async fn chat(&self, prompt: &str, history: &Vec<Message>) -> String {
        let mut attempts = 0;
        let max_attempts = 2;
        loop {
            attempts += 1;
            match self.agent.chat(prompt, history.clone()).await {
                Ok(response) => return response,
                Err(e) => {
                    let err_msg = format!("{}", e);
                    if attempts < max_attempts && (err_msg.contains("CUDA error") || err_msg.contains("500 Internal Server Error")) {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                    return format!("DesktopAgent Error: {}", err_msg);
                }
            }
        }
    }
}
