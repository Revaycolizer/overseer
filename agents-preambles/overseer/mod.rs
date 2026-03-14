use rig::{
    completion::Prompt,
    providers::ollama,
    client::CompletionClient,
};

pub struct OverseerAgent {
    agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl OverseerAgent {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let preamble = "You are the Overseer, a high-precision verification agent for UI automation.
YOUR JOB: Determine if a specific step was ACTUALLY successfully completed based on the Final Grounding and the Plan & History provided.

INPUT:
1. Step Description: The goal the agent was trying to achieve.
2. Final Grounding: The current state of the screen (Vision, UI Tree, OCR).
3. Plan & History: The log of what the agent has done and what the tools returned.

RULES:
- Be skeptical. If the evidence is ambiguous, mark as NOT VERIFIED.
- For 'Open Document' / 'LibreOffice' steps: Verify that 'LibreOffice' or the expected filename (e.g., 'summary.pdf') appears in the App Hierarchy / UI Tree. If a window with that title exists, it is verified.
- For 'Save'/'Export' steps: Verify the file save dialog is gone and the window title is updated or a success message appeared.
- HISTORICAL AWARENESS (CRITICAL): Check the 'Plan & History'. If the last action for this step resulted in an 'Error:', 'failed:', or 'could not be loaded' message, then the step is NOT verified, regardless of what the agent claims.
- DESTINATION RULE (CRITICAL): If the step says 'Click on X' and the FINAL GROUNDING shows that the agent is ALREADY at the destination of X, you MUST output `verified: true`.
- Ignore terminal messages or 'intent' text. Trust ONLY Vision, UI Tree, OCR, and the Reality of Tool Results.

OUTPUT FORMAT:
You MUST output valid JSON only:
{
  \"verified\": bool,
  \"reason\": \"A short explanation of why it is or isn't verified.\"
}";

        let agent = ollama_client.agent("qwen2.5:7b-instruct")
            .preamble(preamble)
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
