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
    _memories: &str,
    _tool: &StructuredPerceptionTool,
    ollama_client: &ollama::Client,
    _timeout: u64,
) -> String {
    let mut master_response = StructuredResponse::new();
    let mut state = StructuredState::new();
    let mut plan_history = String::new();

    for (idx, task) in macro_tasks.iter().enumerate() {
        let mut retry_count = 0;
        let mut domain = task.domain.clone();

        loop {
            // Re-implementing the core loop directly here to avoid missing file dependencies
            match crate::engine::orchestrator::dispatch_domain(
                db, run_id, event_sequence, _all_agents, &domain, &task.task, &task.task, 
                history.to_vec(), &plan_history, &state, ollama_client, 60000, None
            ).await {
                Ok(res) => {
                    let mut compiled = compiler::compile_response(db, &task.task, &res, &state).await.unwrap_or(res);
                    
                    // ARCHITECTURE V3.1: SYNTHESIZER INTEGRATION
                    compiled.summary = synthesizer::synthesize_step_summary(&task.task, &compiled).await;
                    
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

    synthesizer::synthesize_final_response(&mut master_response.messages, &master_response.artifacts);

    let final_json = serde_json::to_string(&master_response).unwrap_or_default();
    println!("📦 FINAL ORCHESTRATOR RESPONSE: {}", final_json);
    final_json
}
