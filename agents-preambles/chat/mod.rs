use rig::{
    agent::AgentBuilder,
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};

pub struct ChatAgent {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl ChatAgent {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let preamble = r#"You are the Chat & Memory Agent.
YOUR JOB:
1. Answer general user questions.
2. Retrieve information from the provided memory context to answer questions about what the agent did in the past.
3. If the user asks what you did, look at the "HISTORICAL REFERENCE DATA" in the memory context.

═══ RULES ═══
1. Be helpful and concise.
2. If you don't know the answer and it's not in memory, say so.
3. Don't hallucinate actions that aren't in the historical data.
"#;

        let agent = AgentBuilder::new(ollama_client.completion_model("llama3:latest"))
            .preamble(preamble)
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
