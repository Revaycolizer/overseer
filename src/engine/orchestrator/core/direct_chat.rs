use crate::db::Database;
use crate::engine::utils::discovery::AgentInfo;
use crate::engine::orchestrator::discovery::dynamic::ChatAgent;
use crate::engine::orchestrator::core::event_logger::record_event;
use crate::engine::orchestrator::core::response::{StructuredResponse, MessagePart};
use rig::providers::ollama;
use rig::completion::Message;

pub async fn handle_direct_chat(
    db: &Database,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    ollama_client: &ollama::Client,
    all_agents: &[AgentInfo],
    clean_query: &str,
    history: &[Message],
    enriched_memory_context: &str,
    decision_details: &str,
) -> String {
    let chat_info = all_agents
        .iter()
        .find(|a| a.domain == "chat")
        .cloned()
        .unwrap_or_else(|| AgentInfo {
            id: 0,
            name: "Chat Assistant".to_string(),
            description: "Direct chat fallback".to_string(),
            tools: Vec::new(),
            instructions: "You are Overseer's chat assistant. Always return your answer as a structured JSON object with this exact shape: {\"messages\": [{\"role\": \"assistant\", \"message\": \"<your direct, conversational answer to the user's question>\"}], \"status\": \"completed\"}. Put your real answer in the inner 'message' string, not in a status or summary field. Do not call any tools; answer the user's question directly.".to_string(),
            model: "qwen2.5:7b-instruct".to_string(),
            domain: "chat".to_string(),
            version: "1.0.0".to_string(),
            code: None,
            source_files: Vec::new(),
            response_format: "json".to_string(),
        });

    let chat_model = crate::backend::resolve_model_name(
        db,
        Some(&chat_info.model),
        "default_chat_model",
        "qwen2.5:7b-instruct",
    )
    .await;

    let chat_agent = ChatAgent::new(ollama_client, &chat_model, &chat_info.instructions);
    let direct_context = if enriched_memory_context.is_empty() {
        "No recent context.".to_string()
    } else {
        enriched_memory_context.to_string()
    };

    record_event(
        db,
        run_id,
        event_sequence,
        "direct_chat_started",
        "running",
        Some("chat"),
        Some(clean_query),
        None,
        Some(&chat_model),
        Some("Routing request directly to chat agent."),
        Some(decision_details),
    )
    .await;

    let raw_response = if history.is_empty() {
        chat_agent.answer(clean_query, &direct_context).await
    } else {
        chat_agent.chat(clean_query, history.to_vec()).await
    };
    
    // ARCHITECTURE V3.4: UNIFORM PROTOCOL ENFORCEMENT
    // If the agent returned a SupervisorExecutionResult JSON, we must parse and format it.
    let response = if raw_response.trim().starts_with('{') && raw_response.trim().ends_with('}') {
        let parsed = crate::engine::orchestrator::dispatch::parsers::parse_supervisor_result(
            raw_response.clone(), 
            false, 
            Some("detailed".to_string())
        );
        crate::engine::orchestrator::utils::formatter::format_supervisor_result(&parsed)
    } else {
        let mut res = StructuredResponse::new();
        res.messages.push(MessagePart {
            role: "assistant".to_string(),
            message: raw_response.clone(),
        });
        res
    };

    record_event(
        db,
        run_id,
        event_sequence,
        "direct_chat_completed",
        "completed",
        Some("chat"),
        Some(clean_query),
        None,
        Some(&chat_model),
        Some("Direct chat completed."),
        None,
    )
    .await;

    let final_json = serde_json::to_string(&response).unwrap_or_default();
    if let Some(rid) = run_id {
        let _ = crate::backend::finish_execution_run(db, rid, "completed", Some(&final_json), None).await;
    }

    final_json
}
