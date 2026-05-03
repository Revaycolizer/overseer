use serde::{Deserialize, Serialize};
use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::providers::ollama;
use rig::completion::Prompt;
use super::StructuredState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub has_conflict: bool,
    pub conflicting_id: Option<String>,
    pub reason: Option<String>,
}

pub async fn check_contradictions(
    ollama_client: &ollama::Client,
    state: &StructuredState,
    new_text: &str,
) -> ConflictReport {
    if state.evidence.is_empty() {
        return ConflictReport { has_conflict: false, conflicting_id: None, reason: None };
    }

    // ARCHITECTURE V3: CONTRADICTION GUARD
    // Use llama3.2:3b to check if the new text contradicts any existing grounded evidence.
    let check_prompt = format!(
        "NEW_CLAIM: {}\n\n\
         EXISTING_EVIDENCE:\n{}\n\n\
         INSTRUCTION: Does the NEW_CLAIM logically contradict any of the EXISTING_EVIDENCE? \
         Contradiction means a direct factual conflict (e.g., different dates, prices, or mutually exclusive events).\
         Respond ONLY with a JSON object: `{{\"has_conflict\": true/false, \"conflicting_id\": \"E#\", \"reason\": \"...\"}}`.",
        new_text, state.to_prompt_string()
    );

    let agent = AgentBuilder::new(ollama_client.completion_model("llama3.2:3b"))
        .build();
    if let Ok(resp) = agent.prompt(&check_prompt).await {
        if let Ok(report) = serde_json::from_str::<ConflictReport>(&resp) {
            return report;
        }
    }

    ConflictReport { has_conflict: false, conflicting_id: None, reason: None }
}
