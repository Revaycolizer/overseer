pub mod planning;

use crate::db::Database;
use crate::engine::orchestrator::core::{direct_chat, event_logger, executor};
use crate::engine::orchestrator::native::native_checks::*;
use crate::engine::orchestrator::planning::{decisions::*, master::MasterPlanner};
use crate::engine::orchestrator::truthfulness;
use crate::engine::orchestrator::utils::context_helpers::*;
use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;
use rig::completion::Message;
use rig::providers::ollama;

struct RunCleanupGuard {
    db: Database,
    run_id: i64,
    completed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for RunCleanupGuard {
    fn drop(&mut self) {
        if !self.completed.load(std::sync::atomic::Ordering::SeqCst) {
            let db = self.db.clone();
            let run_id = self.run_id;
            tokio::spawn(async move {
                let _ = crate::backend::finish_execution_run(
                    &db,
                    run_id,
                    "cancelled",
                    None,
                    Some("Cancelled by user."),
                )
                .await;
                crate::engine::activity::trigger_cancellation(run_id);
                let model = crate::backend::get_setting_or(
                    &db,
                    "default_chat_model",
                    "qwen2.5:7b-instruct",
                )
                .await;
                crate::engine::activity::force_unload_ollama_model(&model).await;
            });
        }
    }
}

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
    token_sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
) -> String {
    let (clean_query, artifact_refs, work_item_refs) = extract_structured_context(query);
    let mut seq = 1_i64;
    let run_id = crate::backend::start_execution_run(db, conversation_id, query)
        .await
        .ok();

    let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _cleanup_guard = run_id.map(|rid| RunCleanupGuard {
        db: db.clone(),
        run_id: rid,
        completed: completed.clone(),
    });

    let learned = crate::engine::learning::context(db, conversation_id, &clean_query)
        .await
        .unwrap_or_default();
    let memory_context = format!("{memory_context}\n{learned}");
    let enriched_memory_context = format!(
        "{}\n\n{}\n\n{}",
        memory_context,
        format_artifact_context(&artifact_refs),
        format_work_item_context(&work_item_refs)
    );

    if let Some((resp, _)) = resolve_screenshot_follow_up(db, conversation_id, &clean_query).await {
        let res = finalize_direct_response(db, run_id, &resp).await;
        completed.store(true, std::sync::atomic::Ordering::SeqCst);
        return res;
    }

    let decision = match decide_orchestration(
        db,
        all_agents,
        &clean_query,
        &history,
        &enriched_memory_context,
        ollama_client,
        None,
        !artifact_refs.is_empty(),
    )
    .await
    {
        Ok(d) => d,
        Err(_) => heuristic_decision(db, &clean_query, None, !artifact_refs.is_empty()).await,
    };

    let decision_details = serde_json::to_string(&decision).unwrap_or_default();
    event_logger::record_event(
        db,
        run_id,
        &mut seq,
        "orchestration_decision",
        "completed",
        decision.domain.as_deref(),
        None,
        None,
        None,
        Some(&format!("Orchestrator chose {:?}.", decision.route)),
        Some(&decision_details),
    )
    .await;

    if matches!(decision.route, OrchestrationRoute::DirectChat) {
        let res = direct_chat::handle_direct_chat(
            db,
            run_id,
            &mut seq,
            ollama_client,
            all_agents,
            &clean_query,
            &history,
            &enriched_memory_context,
            &decision_details,
        )
        .await;
        // Direct chat never runs tools, so any claimed side effect is unverified.
        let res = truthfulness::enforce_truthful_response(&res, false);
        completed.store(true, std::sync::atomic::Ordering::SeqCst);
        return res;
    }

    let macro_tasks = planning::resolve_macro_tasks(
        db,
        master_planner,
        all_agents,
        &clean_query,
        history.clone(),
        &decision,
        ollama_client,
    )
    .await;

    let macro_plan_details = serde_json::to_string(&macro_tasks).unwrap_or_default();
    event_logger::simple(
        db,
        run_id,
        &mut seq,
        "macro_plan",
        "completed",
        &format!("Orchestrator planned {} step(s).", macro_tasks.len()),
        Some(&macro_plan_details),
    )
    .await;

    let res = executor::execute_macro_tasks(
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
        token_sender,
    )
    .await;
    // Truthfulness guard: two-layer verification
    // Layer 1: deterministic side-effect claim detection
    // Layer 2: semantic claim verification against evidence DAG
    let executed_action = crate::backend::run_produced_action(db, run_id).await;
    // Build evidence DAG from the run's execution trace for semantic verification
    let dag = crate::backend::build_evidence_dag_from_run(db, run_id).await;
    let res = truthfulness::verify_response_semantic(&res, executed_action, dag.as_ref());
    if let Err(e) = crate::engine::learning::record_outcome(db, conversation_id, run_id).await {
        tracing::debug!(%e, "Learning outcome was not saved");
    }
    completed.store(true, std::sync::atomic::Ordering::SeqCst);
    res
}

async fn finalize_direct_response(db: &Database, run_id: Option<i64>, resp: &str) -> String {
    let mut sr = crate::engine::orchestrator::core::response::StructuredResponse::new();
    sr.messages
        .push(crate::engine::orchestrator::core::response::MessagePart {
            role: "assistant".to_string(),
            message: resp.to_string(),
        });
    let final_json = serde_json::to_string(&sr).unwrap_or_default();
    if let Some(rid) = run_id {
        let _ = crate::backend::finish_execution_run(db, rid, "completed", Some(&final_json), None)
            .await;
    }
    final_json
}
