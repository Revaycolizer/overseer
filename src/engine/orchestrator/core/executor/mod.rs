pub mod task_loop;
pub mod compiler;

use crate::db::Database;
use crate::engine::orchestrator::core::response::StructuredResponse;
use crate::engine::orchestrator::core::state::StructuredState;
use crate::engine::orchestrator::core::state_update::update_state_from_findings;
use crate::engine::orchestrator::planning::planner::MacroTask;
use crate::engine::utils::discovery::AgentInfo;
use crate::engine::tools::StructuredPerceptionTool;
use rig::completion::Message;
use rig::providers::ollama;

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
        let mut retry_msg = None;
        let mut domain = task.domain.clone();

        loop {
            match task_loop::run_task_iteration(db, run_id, event_sequence, &mut domain, &task.task, &mut state, history, &plan_history, ollama_client, &mut retry_count, &mut retry_msg).await {
                Ok(res) => {
                    let compiled = compiler::compile_response(db, &task.task, &res, &state).await.unwrap_or(res);
                    update_state_from_findings(&mut state, db, &compiled.findings, &compiled.summary, &compiled.evidence).await;
                    plan_history.push_str(&format!("\nStep {}: {}", idx + 1, compiled.summary));
                    master_response.merge(crate::engine::orchestrator::utils::formatter::format_supervisor_result(&compiled));
                    break;
                }
                Err(_) if retry_count <= 2 => continue,
                Err(e) => { master_response.status = "failed".to_string(); break; }
            }
        }
    }

    serde_json::to_string(&master_response).unwrap_or_default()
}
