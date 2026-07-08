pub mod planning;

use crate::db::Database;
use crate::engine::orchestrator::core::{direct_chat, executor, event_logger};
use crate::engine::orchestrator::planning::{master::MasterPlanner, decisions::*};
use crate::engine::utils::discovery::AgentInfo;
use crate::engine::orchestrator::utils::context_helpers::*;
use crate::engine::orchestrator::native::native_checks::*;
use crate::engine::tools::StructuredPerceptionTool;
use rig::providers::ollama;
use rig::completion::Message;

pub async fn orchestrate_request(
    db: &Database,
    conversation_id: i64,
    master_planner: &MasterPlanner,
    all_agents: &[AgentInfo],
    query: &str,
    history: Vec<Message>,
    perception_tool: &StructuredPerceptionTool,
    memory_context: &str,
    ollama_client: &ollama::Client,
) -> String {
    let (clean_query, artifact_refs, work_item_refs) = extract_structured_context(query);
    let mut seq = 1_i64;
    let run_id = crate::backend::start_execution_run(db, conversation_id, query).await.ok();
    
    let enriched_memory_context = format!("{}\n\n{}\n\n{}", memory_context, format_artifact_context(&artifact_refs), format_work_item_context(&work_item_refs));

    if let Some((resp, _)) = resolve_screenshot_follow_up(db, conversation_id, &clean_query).await {
        return finalize_direct_response(db, run_id, &resp).await;
    }

    let decision = match decide_orchestration(db, all_agents, &clean_query, &history, &enriched_memory_context, ollama_client, None, !artifact_refs.is_empty()).await {
        Ok(d) => d,
        Err(_) => heuristic_decision(db, &clean_query, None, !artifact_refs.is_empty()).await,
    };

    let decision_details = serde_json::to_string(&decision).unwrap_or_default();
    event_logger::record_event(db, run_id, &mut seq, "orchestration_decision", "completed", decision.domain.as_deref(), None, None, None, Some(&format!("Orchestrator chose {:?}.", decision.route)), Some(&decision_details)).await;

    if matches!(decision.route, OrchestrationRoute::DirectChat) {
        return direct_chat::handle_direct_chat(db, run_id, &mut seq, ollama_client, all_agents, &clean_query, &history, &enriched_memory_context, &decision_details).await;
    }

    let macro_tasks = planning::resolve_macro_tasks(db, master_planner, all_agents, &clean_query, history.clone(), &decision, ollama_client).await;

    let macro_plan_details = serde_json::to_string(&macro_tasks).unwrap_or_default();
    event_logger::simple(db, run_id, &mut seq, "macro_plan", "completed", &format!("Orchestrator planned {} step(s).", macro_tasks.len()), Some(&macro_plan_details)).await;

    executor::execute_macro_tasks(db, conversation_id, run_id, &mut seq, all_agents, macro_tasks, &clean_query, &history, &enriched_memory_context, perception_tool, ollama_client, 60000).await
}

async fn finalize_direct_response(db: &Database, run_id: Option<i64>, resp: &str) -> String {
    let mut sr = crate::engine::orchestrator::core::response::StructuredResponse::new();
    sr.messages.push(crate::engine::orchestrator::core::response::MessagePart { role: "assistant".to_string(), message: resp.to_string() });
    let final_json = serde_json::to_string(&sr).unwrap_or_default();
    if let Some(rid) = run_id { let _ = crate::backend::finish_execution_run(db, rid, "completed", Some(&final_json), None).await; }
    final_json
}
