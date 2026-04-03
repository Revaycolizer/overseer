use crate::db::Database;
use crate::engine::orchestrator::core::{event_logger::record_event, response::StructuredResponse, response::MessagePart};
use crate::engine::orchestrator::planning::planner::MacroTask;
use crate::engine::utils::discovery::AgentInfo;
use crate::engine::orchestrator::dispatch;
use crate::engine::tools::StructuredPerceptionTool;
use rig::completion::Message;
use rig::providers::ollama;
use crate::engine::orchestrator::utils::context_helpers::{format_newly_created_artifacts, format_step_results_context};

pub async fn execute_macro_tasks(
    db: &Database,
    conversation_id: i64,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    all_agents: &[AgentInfo],
    macro_tasks: Vec<MacroTask>,
    clean_query: &str,
    history: &[Message],
    memories: &str,
    perception_tool: &StructuredPerceptionTool,
    ollama_client: &ollama::Client,
    model_timeout_ms: u64,
) -> String {
    let mut master_response = StructuredResponse::new();
    let mut plan_history = String::new();
    let mut active_memories = memories.to_string();
    let mut browser_pid: Option<i32> = None;

    for (task_idx, macro_task) in macro_tasks.iter().enumerate() {
        println!(
            "\n🚀 EXECUTING MACRO TASK {}/{}: [{}] {}...",
            task_idx + 1,
            macro_tasks.len(),
            macro_task.domain,
            macro_task.task
        );

        record_event(
            db,
            run_id,
            event_sequence,
            "macro_task_started",
            "running",
            Some(&macro_task.domain),
            Some(&macro_task.task),
            None,
            None,
            Some("Executing macro task."),
            None,
        )
        .await;

        let micro_steps = vec![macro_task.task.clone()]; // Simplified for restoration

        for (i, step) in micro_steps.iter().enumerate() {
            println!("\n▶️ EXECUTING MICRO-STEP {}/{}...", i + 1, micro_steps.len());

            record_event(
                db,
                run_id,
                event_sequence,
                "micro_step_started",
                "running",
                Some(&macro_task.domain),
                Some(step),
                None,
                None,
                Some("Executing micro-step."),
                None,
            )
            .await;

            match dispatch::dispatch_domain(
                db,
                run_id,
                event_sequence,
                all_agents,
                &macro_task.domain,
                step,
                clean_query,
                history.to_vec(),
                &plan_history,
                &active_memories,
                ollama_client,
                model_timeout_ms,
            )
            .await
            {
                Ok(r) => {
                    let supervisor_result: crate::engine::orchestrator::core::result::SupervisorExecutionResult = r;
                    let step_structured = crate::engine::orchestrator::utils::formatter::format_supervisor_result(&supervisor_result);
                    master_response.merge(step_structured);
                    
                    if !supervisor_result.artifacts.is_empty() {
                        let new_ctx = format_newly_created_artifacts(&supervisor_result.artifacts, step);
                        active_memories.push_str("\n\n");
                        active_memories.push_str(&new_ctx);
                    }

                    // ADDITION: Also pass textual findings (summary) as structured context
                    let findings_ctx = format_step_results_context(step, &supervisor_result.summary, &supervisor_result.evidence, &supervisor_result.findings);
                    if !findings_ctx.is_empty() {
                        println!("🧠 CONTEXT ACCUMULATED for next step: {}", findings_ctx);
                        active_memories.push_str("\n\n");
                        active_memories.push_str(&findings_ctx);
                    }

                    let display = supervisor_result.display_text();
                    plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i + 1, step, display));

                    record_event(
                        db,
                        run_id,
                        event_sequence,
                        "micro_step_completed",
                        "completed",
                        Some(&macro_task.domain),
                        Some(step),
                        None,
                        None,
                        Some(&display),
                        None,
                    )
                    .await;
                }
                Err(e) => {
                    let error_msg = format!("Task failed: {}", e);
                    master_response.messages.push(MessagePart {
                        message: format!("❌ ERROR: {}", error_msg),
                        role: "assistant".to_string(),
                    });
                    record_event(
                        db,
                        run_id,
                        event_sequence,
                        "micro_step_failed",
                        "error",
                        Some(&macro_task.domain),
                        Some(step),
                        None,
                        None,
                        Some(&error_msg),
                        None,
                    )
                    .await;
                }
            }
        }
    }

    let final_report = serde_json::to_string(&master_response).unwrap_or_else(|_| "{\"status\":\"failed\"}".to_string());
    if let Some(rid) = run_id {
        let _ = crate::backend::finish_execution_run(db, rid, "completed", Some(&final_report), None).await;
    }

    final_report
}
