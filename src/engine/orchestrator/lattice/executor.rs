use crate::db::Database;
use crate::engine::orchestrator::core::response::StructuredResponse;
use crate::engine::orchestrator::state::StructuredState;
use crate::engine::orchestrator::state::update_state_from_findings;
use crate::engine::orchestrator::planning::planner::MacroTask;
use crate::engine::utils::discovery::AgentInfo;
use crate::engine::tools::StructuredPerceptionTool;
use rig::completion::Message;
use rig::providers::ollama;
use crate::engine::orchestrator::lattice::compiler;
use crate::engine::orchestrator::lattice::synthesizer;
pub async fn execute_macro_tasks(
    db: &Database,
    conversation_id: i64,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    _all_agents: &[AgentInfo],
    macro_tasks: Vec<MacroTask>,
    _query: &str,
    history: &[Message],
    memories: &str,
    _tool: &StructuredPerceptionTool,
    ollama_client: &ollama::Client,
    _timeout: u64,
) -> String {
    let mut master_response = StructuredResponse::new();
    let mut state = StructuredState::new();
    state.add_legacy_context(memories);
    let mut plan_history = String::new();

    for (idx, task) in macro_tasks.iter().enumerate() {
        let mut retry_count = 0;
        let mut domain = task.domain.clone();

        loop {
            // Fetch supervisor config to get dynamic synthesis flags
            let supervisor = sqlx::query_as::<_, crate::db::Supervisor>(
                "SELECT * FROM supervisors WHERE domain = ?"
            )
            .bind(&domain)
            .fetch_optional(&db.pool)
            .await
            .unwrap_or(None);

            let (hide_evidence, report_style) = if let Some(ref s) = supervisor {
                (s.hide_evidence, s.report_style.clone().unwrap_or_else(|| "detailed".to_string()))
            } else {
                (false, "detailed".to_string())
            };

            // Re-implementing the core loop directly here to avoid missing file dependencies
            match crate::engine::orchestrator::dispatch_domain(
                db, run_id, event_sequence, _all_agents, &domain, &task.task, &task.task, 
                history.to_vec(), &plan_history, &state, ollama_client, 60000, None
            ).await {
                Ok(res) => {
                    let mut compiled = compiler::compile_response(db, &task.task, &res, &state).await.unwrap_or(res);
                    
                    // ARCHITECTURE V3.1: DYNAMIC AGENT-BASED SYNTHESIS
                    compiled.hide_evidence = hide_evidence;
                    
                    let bypass = supervisor.as_ref().map(|s| s.bypass_synthesizer).unwrap_or(false);
                    // REDUNDANCY CHECK: Skip LLM synthesis if artifacts were generated (Final synthesizer handles it)
                    if !bypass && compiled.artifacts.is_empty() {
                        synthesizer::synthesize_step_result(db, ollama_client, &mut compiled, &report_style, &task.task).await;
                    }
                    
                    update_state_from_findings(&mut state, db, &compiled.findings, &compiled.summary, &compiled.evidence).await;
                    plan_history.push_str(&format!("\nStep {}: {}", idx + 1, compiled.summary));
                    
                    master_response.merge(crate::engine::orchestrator::utils::formatter::format_supervisor_result(&compiled));
                    break;
                }
                Err(_) if retry_count <= 2 => {
                    retry_count += 1;
                    continue;
                },
                Err(_) => { 
                    master_response.status = "failed".to_string(); 
                    break; 
                }
            }
        }
    }

    // Final Post-Synthesis for complex multi-step plans (Centralized UI Governance)
    let has_research = macro_tasks.iter().any(|t| matches!(t.domain.as_str(), "stealth" | "web"));
    synthesizer::synthesize_final_response(db, ollama_client, history.to_vec(), &mut master_response, has_research, _query).await;

    let final_json = serde_json::to_string(&master_response).unwrap_or_default();
    println!("📦 FINAL ORCHESTRATOR RESPONSE: {}", final_json);
    final_json
}
