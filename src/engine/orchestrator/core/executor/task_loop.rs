use crate::db::Database;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::state::StructuredState;
use crate::engine::orchestrator::core::state_update::update_state_from_findings;
use crate::engine::orchestrator::dispatch;
use rig::completion::Message;
use rig::providers::ollama;

pub async fn run_task_iteration(
    db: &Database,
    run_id: Option<i64>,
    seq: &mut i64,
    domain: &mut String,
    step: &str,
    state: &mut StructuredState,
    history: &[Message],
    plan_history: &str,
    ollama_client: &ollama::Client,
    retry_count: &mut i32,
    retry_msg: &mut Option<String>,
) -> Result<SupervisorExecutionResult, String> {
    let res = dispatch::dispatch_domain(db, run_id, seq, &[], domain, step, "", history.to_vec(), plan_history, state, ollama_client, 60000, retry_msg.as_deref()).await?;
    
    if res.status == "stalled" && *retry_count < 2 {
        *retry_count += 1;
        *retry_msg = Some("Your response was rejected. CALL A TOOL NOW.".to_string());
        // Pivot to office if needed
        if *retry_count == 2 && step.to_lowercase().contains("pdf") && domain != "office" {
            *domain = "office".to_string();
        }
        return Err("stalled".to_string());
    }
    
    Ok(res)
}
