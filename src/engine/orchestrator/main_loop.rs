use crate::db::Database;
use crate::engine::orchestrator::*;
use crate::engine::tools::StructuredPerceptionTool;
use rig::providers::ollama;
use rig::completion::Message;
use std::time::Duration;
use tokio::time::timeout;

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
    
    let artifact_context = format_artifact_context(&artifact_refs);
    let work_item_context = format_work_item_context(&work_item_refs);
    let enriched_memory_context = format!("{}\n\n{}\n\n{}", memory_context, artifact_context, work_item_context);

    if let Some((resp, _)) = resolve_screenshot_follow_up(db, conversation_id, &clean_query).await {
        if let Some(rid) = run_id {
            let _ = crate::backend::finish_execution_run(db, rid, "completed", Some(&resp), None).await;
        }
        return resp;
    }

    let decision = match decide_orchestration(db, all_agents, &clean_query, &history, &enriched_memory_context, ollama_client, None, !artifact_refs.is_empty()).await {
        Ok(d) => d,
        Err(_) => heuristic_decision(db, &clean_query, None, !artifact_refs.is_empty()).await,
    };

    let decision_details = serde_json::to_string(&decision).unwrap_or_default();
    event_logger::record_event(db, run_id, &mut seq, "orchestration_decision", "completed", decision.domain.as_deref(), None, None, None, Some(&format!("Orchestrator chose {:?} with personality '{}'.", decision.route, decision.personality_mode)), Some(&decision_details)).await;

    if matches!(decision.route, OrchestrationRoute::DirectChat) {
        return direct_chat::handle_direct_chat(db, run_id, &mut seq, ollama_client, all_agents, &clean_query, &enriched_memory_context, &decision_details).await;
    }

    let macro_tasks = if matches!(decision.route, OrchestrationRoute::SingleDomain) {
        let synthetic = serde_json::json!([{"domain": decision.domain.unwrap_or_else(|| "chat".to_string()), "task": clean_query}]).to_string();
        planner_utils::sanitize_plan(&synthetic, &clean_query)
    } else {
        match timeout(Duration::from_millis(90000), master_planner.plan(&clean_query)).await {
            Ok(plan) => planner_utils::sanitize_plan(&plan, &clean_query),
            Err(_) => return "Planner timed out.".to_string(),
        }
    };

    executor::execute_macro_tasks(
        db,
        conversation_id,
        run_id,
        &mut seq,
        all_agents,
        macro_tasks,
        &clean_query,
        &history,
        &enriched_memory_context,
        perception_tool,
        ollama_client,
        60000,
    ).await
}
