use crate::db::Database;
use crate::engine::orchestrator::core::response::{MessagePart, StructuredResponse};
use rig::providers::ollama::{self, OllamaExt};
use rig::client::CompletionClient;
use rig::agent::AgentBuilder;
use rig::completion::{Chat, Prompt};
use serde_json::Value;

pub async fn synthesize_final_response(
    db: &Database,
    ollama_client: &ollama::Client,
    history: Vec<rig::completion::Message>,
    response: &mut StructuredResponse,
    _has_research: bool,
    query: &str,
) {
    let (instructions, model_name) = match sqlx::query(
        "SELECT instructions, model FROM agents WHERE domain = 'synthesizer' LIMIT 1"
    )
    .fetch_optional(&db.pool).await {
        Ok(Some(row)) => {
            use sqlx::Row;
            (row.get::<Option<String>, _>("instructions").unwrap_or_default(),
             row.get::<Option<String>, _>("model").unwrap_or_else(|| "qwen2.5:7b-instruct".to_string()))
        }
        _ => ("Professional synthesizer.".to_string(), "qwen2.5:7b-instruct".to_string())
    };

    let prompt = format!(
        "USER_QUERY: {}\nCURRENT_RESPONSE: {}\n\n\
        ARCHITECTURE V3.6: CONVERSATIONAL ALIGNMENT
        1. Final Summary: If the response is creative (story, greeting, etc.), PRESERVE its essence. 
           If it is operational (results/data), professionally summarize it based ONLY on CURRENT_RESPONSE.
        2. UI Pruning: STRIP ALL internal artifacts like \"NEXT STEPS:\", \"PLAN:\", \"Internal Reasoning:\", and \"Vision Insights:\".
        3. Accuracy First: If the USER_QUERY is a specific question, respond ONLY with the answer. Do NOT provide generic media descriptions unless requested.
        4. Grounding Mandate: If CURRENT_RESPONSE has sources, charts, or artifacts, YOU MUST acknowledge or summarize them. NEVER say 'no data found' if these fields are populated.
        
        Output JSON: {{\"message\": \"...\", \"keep_sources\": bool, \"keep_attachments\": bool}}",
        query, serde_json::to_string(response).unwrap_or_default()
    );

    let agent = AgentBuilder::new(ollama_client.completion_model(&model_name))
        .preamble(&instructions).build();

    if let Ok(res_text) = agent.chat(prompt, history).await {
        if let Ok(v) = serde_json::from_str::<Value>(&res_text) {
            if let Some(msg) = v.get("message").and_then(|m| m.as_str()) {
                response.messages.clear();
                response.messages.push(MessagePart { role: "assistant".to_string(), message: msg.to_string() });
            }
            if v.get("keep_sources").and_then(|k| k.as_bool()) == Some(false) { response.sources.clear(); }
            if v.get("keep_attachments").and_then(|k| k.as_bool()) == Some(false) {
                response.attachments.clear(); response.artifacts.clear();
            }
        }
    }
}
