use crate::db::Database;
use crate::engine::orchestrator::core::{direct_chat, executor, event_logger};
use crate::engine::orchestrator::planning::{master::MasterPlanner, decisions::*, planner_utils};
use crate::engine::orchestrator::discovery::discovery;
use crate::engine::utils::discovery::AgentInfo;
use crate::engine::orchestrator::utils::{context_helpers::*, artifact_utils::*};
use crate::engine::orchestrator::native::native_checks::*;
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
        let mut sr = crate::engine::orchestrator::core::response::StructuredResponse::new();
        sr.messages.push(crate::engine::orchestrator::core::response::MessagePart {
            role: "assistant".to_string(),
            message: resp.clone(),
        });
        let final_json = serde_json::to_string(&sr).unwrap_or_default();
        if let Some(rid) = run_id {
            let _ = crate::backend::finish_execution_run(db, rid, "completed", Some(&final_json), None).await;
        }
        return final_json;
    }

    let decision = match decide_orchestration(db, all_agents, &clean_query, &history, &enriched_memory_context, ollama_client, None, !artifact_refs.is_empty()).await {
        Ok(d) => d,
        Err(_) => heuristic_decision(db, &clean_query, None, !artifact_refs.is_empty()).await,
    };

    let decision_details = serde_json::to_string(&decision).unwrap_or_default();
    event_logger::record_event(db, run_id, &mut seq, "orchestration_decision", "completed", decision.domain.as_deref(), None, None, None, Some(&format!("Orchestrator chose {:?} with personality '{}'.", decision.route, decision.personality_mode)), Some(&decision_details)).await;

    if matches!(decision.route, OrchestrationRoute::DirectChat) {
        return direct_chat::handle_direct_chat(db, run_id, &mut seq, ollama_client, all_agents, &clean_query, &history, &enriched_memory_context, &decision_details).await;
    }

    let macro_tasks = if matches!(decision.route, OrchestrationRoute::SingleDomain) {
        let synthetic = serde_json::json!([{"domain": decision.domain.unwrap_or_else(|| "chat".to_string()), "task": clean_query}]).to_string();
        planner_utils::sanitize_plan(&synthetic, &clean_query)
    } else {
        // Dynamic Agent Discovery
        let http_client = reqwest::Client::new();
        let discovered_agents = discovery::discover_semantic_agents(db, &http_client, &clean_query, 8, ollama_client).await;
        
        let mut macro_tasks = match timeout(Duration::from_millis(90000), master_planner.plan(ollama_client, &clean_query, history.clone(), discovered_agents.clone())).await {
            Ok(plan) => planner_utils::sanitize_plan(&plan, &clean_query),
            Err(_) => {
                let mut error_res = crate::engine::orchestrator::core::response::StructuredResponse::new();
                error_res.status = "failed".to_string();
                error_res.messages.push(crate::engine::orchestrator::core::response::MessagePart {
                    role: "assistant".to_string(),
                    message: "❌ Error: The planner timed out while trying to coordinate your request. Please try again or simplify your query.".to_string(),
                });
                return serde_json::to_string(&error_res).unwrap_or_else(|_| "{\"status\":\"failed\"}".to_string());
            }
        };

        // ARCHITECTURE V2.7: DECOMPOSITION INTEGRITY CHECK
        // If we forced MultiStep but the planner gave a 1-step plan for an artifact task, force a re-plan.
        let lower_query = clean_query.to_lowercase();
        // Only force Research→Generation decomposition for explicit document FORMAT requests.
        // 'chart' and 'document' are intentionally excluded — they don't require the office pipeline.
        let artifact_keywords = ["pdf", "docx", "xlsx", "csv", "report"];
        let is_artifact_task = artifact_keywords.iter().any(|&kw| lower_query.contains(kw));

        if is_artifact_task && macro_tasks.len() == 1 {
            println!("⚖️ DECOMPOSITION FAILURE detected. Forcing autonomous re-plan with strict mandate.");
            let retry_prompt = format!(
                "{}\n\n⚠️ COORDINATION FAILURE: You provided a single-step plan for a complex artifact task. \
                 You MUST decompose this into at least TWO steps: \
                 1. Information Gathering (stealth or web domain) \
                 2. Document Generation (office domain).",
                clean_query
            );
            
            if let Ok(plan) = timeout(Duration::from_millis(60000), master_planner.plan(ollama_client, &retry_prompt, history.clone(), discovered_agents)).await {
                macro_tasks = planner_utils::sanitize_plan(&plan, &clean_query);
            }

            // FINAL SILICON GUARDRAIL: If STILL 1-step, force synthetic decomposition
            if macro_tasks.len() == 1 {
                macro_tasks = planner_utils::force_decompose_artifact_plan(&clean_query);
            }
        }
        macro_tasks
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
