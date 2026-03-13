use rig::{
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};

#[derive(Clone)]
pub struct OverseerAgent {
    agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl OverseerAgent {
    pub fn new(ollama_client: &ollama::Client, instructions: &str) -> Self {
        let agent = ollama_client.agent("qwen2.5:7b-instruct")
            .preamble(instructions)
            .build();
        
        Self { agent }
    }

    pub async fn verify(&self, step: &str, grounding: &str, plan_history: &str) -> (bool, String) {
        let prompt = format!(
            "STEP TO VERIFY: \"{}\"\n\nPLAN & HISTORY:\n{}\n\nFINAL GROUNDING:\n{}\n\nDoes the plan and grounding prove the step is completed? Respond in JSON.",
            step, plan_history, grounding
        );

        match self.agent.prompt(&prompt).await {
            Ok(response) => {
                let clean = response.replace("```json", "").replace("```", "");
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&clean) {
                    let verified = val.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
                    let reason = val.get("reason").and_then(|v| v.as_str()).unwrap_or("No reason provided.").to_string();
                    (verified, reason)
                } else {
                    (false, format!("Failed to parse Overseer JSON: {}", response))
                }
            }
            Err(e) => (false, format!("Overseer error: {}", e)),
        }
    }
}
