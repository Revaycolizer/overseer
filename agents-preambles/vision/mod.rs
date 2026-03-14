use rig::{
    completion::{Message, Chat},
    client::CompletionClient,
    providers::ollama,
};
use crate::tools::VisionTool;

pub struct VisionGeneralAgent {
    agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl VisionGeneralAgent {
    pub fn new(client: &ollama::Client) -> Self {
        let agent = client
            .agent("qwen2.5:7b-instruct") // Using stronger model for reasoning
            .preamble(
                "You are a Vision and General Reasoning Assistant.
                 
                 Your capabilities:
                 1. Analyze images (vision_analysis).
                 2. Answer general questions and reason about complex topics.
                 
                 When asked about an image, ALWAYS use the `vision_analysis` tool.
                 For general questions, use your internal knowledge.",
            )
            .tool(VisionTool { client: client.clone() })
            .default_max_turns(10)
            // GeneralTool is likely redundant if the agent itself is the general reasoner, 
            // but keeping it if it wraps a specific specific capability or different model.
            // For now, simple prompting is enough for general reasoning.
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
                    if attempts < max_attempts && (err_msg.contains("CUDA error") || err_msg.contains("500 Internal Server Error") || err_msg.contains("ggml")) {
                        eprintln!("⚠️ CUDA/Ollama error detected (Vision). Retrying in 2s (attempt {}/{})...", attempts, max_attempts);
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                    return format!("VisionGeneralAgent Error: {}", err_msg);
                }
            }
        }
    }
}
