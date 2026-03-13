use rig::{
    agent::AgentBuilder,
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};

#[derive(Clone)]
pub struct DesktopPlanner {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl DesktopPlanner {
    pub fn new(ollama_client: &ollama::Client, instructions: &str) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("llama3:latest"))
            .preamble(instructions)
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
