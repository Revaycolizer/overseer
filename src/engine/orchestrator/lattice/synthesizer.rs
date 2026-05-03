use crate::db::Database;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::response::{MessagePart, StructuredResponse};
use crate::engine::orchestrator::utils::format_helpers::{sanitize_links, strip_grounding_markers};
use rig::providers::ollama::{self, OllamaExt};
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::agent::AgentBuilder;

pub async fn synthesize_step_result(
    db: &Database,
    ollama_client: &ollama::Client,
    result: &mut SupervisorExecutionResult,
    report_style: &str,
    task_description: &str,
) {
    if result.hide_evidence {
        result.evidence.clear();
    }
    
    let (instructions, model_name) = match sqlx::query(
        "SELECT instructions, model FROM agents WHERE domain = 'synthesizer' LIMIT 1"
    )
    .fetch_optional(&db.pool)
    .await
    {
        Ok(Some(row)) => {
            use sqlx::Row;
            (
                row.get::<Option<String>, _>("instructions").unwrap_or_default(),
                row.get::<Option<String>, _>("model").unwrap_or_else(|| "qwen2.5:7b-instruct".to_string())
            )
        }
        _ => (
            "You are a professional assistant. Synthesize a 1-sentence summary of the task completion.".to_string(),
            "qwen2.5:7b-instruct".to_string()
        )
    };

    let findings_json = serde_json::to_string(&result.findings).unwrap_or_default();
    let artifacts_list = result.artifacts.join(", ");
    
    let prompt = format!(
        "{}\n\nUSER INTENT: {}\nFINDINGS: {}\nARTIFACTS: {}\nSTYLE: {}\n\nSYNTHESIZE A 1-SENTENCE PROFESSIONAL SUMMARY:",
        instructions, task_description, findings_json, artifacts_list, report_style
    );

    let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model(&model_name))
        .preamble(&instructions)
        .build();

    let response_result: Result<String, _> = agent.prompt(&prompt).await;
    if let Ok(response) = response_result {
        let clean_response = strip_grounding_markers(&response);
        result.summary = sanitize_links(&clean_response);
    } else {
        
        let raw = strip_grounding_markers(&result.summary);
        result.summary = sanitize_links(&raw);
    }
}

pub fn synthesize_final_response(
    response: &mut StructuredResponse,
    has_research: bool,
) {
    if !response.attachments.is_empty() || !response.artifacts.is_empty() {
        let items = if !response.attachments.is_empty() { &response.attachments } else { &response.artifacts };
        
        response.messages.clear();
        response.messages.push(MessagePart {
            role: "agent".to_string(),
            message: format!("I have successfully completed your request. The following attachments are ready"),
        });

        if !has_research {
            response.sources.clear();
        }
    }
}
