use rig::{
    agent::Agent,
    agent::AgentBuilder,
    client::CompletionClient,
    completion::{Chat, Message, Prompt},
    providers::ollama,
    message::UserContent,
};
use base64::{engine::general_purpose, Engine as _};

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
            "USER_REQUEST: {}\n\nPLAN_HISTORY:\n{}\n\nCURRENT_GROUNDING:\n{}\n\n\
             Has the user's specific request been fully completed? \
             Respond with exactly: 'YES' or 'NO: [reason]'",
            query, history, grounding
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

    pub async fn chat(&self, prompt: &str, history: Vec<Message>) -> String {
        self.agent
            .chat(prompt, history)
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

    pub async fn perceive(&self, prompt: &str, image_path: &std::path::Path) -> String {
        let image_data = match std::fs::read(image_path) {
            Ok(data) => data,
            Err(e) => return format!("Failed to read image for vision: {}", e),
        };

        let b64 = general_purpose::STANDARD.encode(image_data);
        let ext = image_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let media_type = match ext.as_str() {
            "png" => Some(rig::message::ImageMediaType::PNG),
            "webp" => Some(rig::message::ImageMediaType::WEBP),
            "gif" => Some(rig::message::ImageMediaType::GIF),
            _ => Some(rig::message::ImageMediaType::JPEG),
        };

        let content = vec![
            UserContent::text(prompt.to_string()),
            UserContent::Image(rig::completion::message::Image {
                data: rig::message::DocumentSourceKind::Base64(b64),
                media_type,
                detail: None,
                additional_params: None,
            }),
        ];

        self.agent
            .prompt(rig::completion::Message::User { 
                content: rig::OneOrMany::many(content).expect("Non-empty contents") 
            })
            .await
            .unwrap_or_else(|e| format!("Vision perception error: {}", e))
    }
}
