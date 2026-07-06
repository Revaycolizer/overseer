use crate::db::Database;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::response::{MessagePart, StructuredResponse};
use crate::engine::orchestrator::utils::format_helpers::{sanitize_links, strip_grounding_markers};
use rig::providers::ollama::{self, OllamaExt};
use rig::client::CompletionClient;
use rig::agent::AgentBuilder;
use rig::completion::Prompt;
use serde_json::Value;

pub async fn synthesize_step_result(
    db: &Database,
    ollama_client: &ollama::Client,
    result: &mut SupervisorExecutionResult,
    report_style: &str,
    task_description: &str,
) {
    if result.hide_evidence && matches!(report_style, "help" | "delivery" | "artifact") {
        result.evidence.clear();
    }
    
    let (instructions, model_name) = match sqlx::query(
        "SELECT instructions, model FROM agents WHERE domain = 'synthesizer' LIMIT 1"
    )
    .fetch_optional(&db.pool)
    .await {
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

    let prompt = format!(
        "INSTRUCTION: Synthesize a professional, 1-sentence summary based STRICTLY on the STEP_RESULT. Output ONLY the summary text, with no introductory phrases.\n\n\
         STYLE: {}\n\
         USER INTENT: {}\n\
         STEP_RESULT: {}",
        report_style, task_description,
        serde_json::json!({
            "findings": result.findings,
            "charts": result.charts,
            "artifacts": result.artifacts,
            "evidence": result.evidence,
            "messages": result.messages
        })
    );

    let agent = AgentBuilder::new(ollama_client.completion_model(&model_name))
        .preamble(&instructions).build();

    if let Ok(res_text) = agent.prompt(prompt).await {
        let clean = strip_grounding_markers(&res_text)
            .trim_start_matches("Professional Summary:")
            .trim_start_matches("Summary:").trim().to_string();
        result.summary = sanitize_links(&clean);
    } else {
        result.summary = sanitize_links(&strip_grounding_markers(&result.summary));
    }
}
