use rig::{
    agent::AgentBuilder,
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};

#[derive(Clone)]
pub struct ChatAgent {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl ChatAgent {
    pub fn new(ollama_client: &ollama::Client, instructions: &str) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("llama3:latest"))
            .preamble(instructions)
            .default_max_turns(1)
            .build();
        Self { agent }
    }

    pub async fn answer(&self, query: &str, memory_context: &str) -> String {
        let prompt = format!(
            "{}\n\nUser Question: {}",
            memory_context, query
        );

        self.agent
            .prompt(&prompt)
            .await
            .unwrap_or_else(|e| format!("Chat error: {}", e))
    }
}
