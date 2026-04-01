use rig::{
    agent::Agent,
    agent::AgentBuilder,
    client::CompletionClient,
    completion::{Chat, Message, Prompt},
    providers::ollama,
};

#[derive(Clone)]
pub struct OverseerAgent {
    agent: Agent<ollama::CompletionModel>,
}

impl OverseerAgent {
    pub fn new(ollama_client: &ollama::Client, model: &str, instructions: &str) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model(model))
            .preamble(instructions)
            .build();
        Self { agent }
    }

    pub async fn verify(&self, query: &str, grounding: &str, history: &str) -> (bool, String) {
        let prompt = format!(
            "USER_REQUEST: {}\n\nCURRENT_GROUNDING:\n{}\n\nPLAN_HISTORY:\n{}\n\n\
             Has the user's specific request been fully completed? \
             Respond with exactly: 'YES' or 'NO: [reason]'",
            query, grounding, history
        );

        match self.agent.prompt(&prompt).await {
            Ok(resp) => {
                let resp = resp.trim().to_uppercase();
                if resp.starts_with("YES") {
                    (true, "Verified by Overseer.".to_string())
                } else {
                    (false, resp.replace("NO:", "").trim().to_string())
                }
            }
            Err(e) => (false, format!("Overseer error: {}", e)),
        }
    }
}

#[derive(Clone)]
pub struct ChatAgent {
    agent: Agent<ollama::CompletionModel>,
}

impl ChatAgent {
    pub fn new(ollama_client: &ollama::Client, model: &str, instructions: &str) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model(model))
            .preamble(instructions)
            .build();
        Self { agent }
    }

    pub async fn answer(&self, prompt: &str, context: &str) -> String {
        let full_prompt = format!("Context:\n{}\n\nUser Question: {}", context, prompt);
        self.agent
            .prompt(&full_prompt)
            .await
            .unwrap_or_else(|e| format!("Chat error: {}", e))
    }
}

#[derive(Clone)]
pub struct VisionGeneralAgent {
    agent: Agent<ollama::CompletionModel>,
}

impl VisionGeneralAgent {
    pub fn new(ollama_client: &ollama::Client, model: &str, instructions: &str) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model(model))
            .preamble(instructions)
            .build();
        Self { agent }
    }

    pub async fn chat(&self, prompt: &str, history: &[Message]) -> String {
        self.agent
            .chat(prompt, history.to_vec())
            .await
            .unwrap_or_else(|e| format!("Vision error: {}", e))
    }
}
