use rig::agent::AgentBuilder;
use rig::completion::{Message, Prompt, PromptError};
use rig::providers::ollama;
use rig::prelude::CompletionClient;
use super::types::OrchestrationDecision;

pub async fn decide_orchestration_with_llm(
    ollama_client: &ollama::Client,
    model: &str,
    query: &str,
    history: &[Message],
    memory_context: &str,
) -> Result<OrchestrationDecision, String> {
    let agent = AgentBuilder::new(ollama_client.completion_model(model))
        .preamble("You are the Overseer Router. JSON output only: {\"route\": \"direct_chat|single_domain|multi_step\", \"domain\": \"domain_name_or_null\", \"personality_mode\": \"assistant|operator|orchestrator\", \"reason\": \"explanation\"}")
        .build();

    let history_str = history.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>().join("\n");
    let prompt = format!("CONTEXT: {}\nHISTORY: {}\nQUERY: {}\nDecide route.", memory_context, history_str, query);

    let response = agent.prompt(&prompt).await.map_err(|e: PromptError| e.to_string())?;
    let trimmed = response.trim();
    let json_str = if let Some(s) = trimmed.find('{') { if let Some(e) = trimmed.rfind('}') { &trimmed[s..=e] } else { trimmed } } else { trimmed };

    serde_json::from_str(json_str).map_err(|e| format!("Router JSON error: {}", e))
}
