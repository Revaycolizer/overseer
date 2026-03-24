use crate::db::Database;
use crate::engine::orchestrator::dispatcher::SupervisorExecutionResult;
use crate::engine::utils::discovery::AgentInfo;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::ollama;
use std::time::Duration;
use tokio::time::timeout;

pub async fn verify_execution_grounding(
    db: &Database,
    all_agents: &[AgentInfo],
    ollama_client: &ollama::Client,
    query: &str,
    step: &str,
    macro_task_domain: &str,
    mut supervisor_result: SupervisorExecutionResult,
    model_timeout_ms: u64,
) -> Result<SupervisorExecutionResult, String> {
    // 1. Resolve Supervisor to see if they have a custom verification agent
    let supervisor = sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT * FROM supervisors WHERE domain = ?"
    )
    .bind(macro_task_domain)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| format!("DB error loading supervisor for grounding: {}", e))?;

    let verification_agent = if let Some(ref s) = supervisor {
        if let Some(agent_id) = s.verification_agent_id {
            all_agents.iter().find(|a| a.id == agent_id).cloned()
        } else {
            // Fallback for system domains
            let interactive_domains = ["web", "desktop", "computer", "neontune", "app"];
            if interactive_domains.contains(&macro_task_domain) {
                all_agents.iter().find(|a| a.name == "Grounding Agent").cloned()
            } else {
                None
            }
        }
    } else {
        None
    };

    if supervisor_result.status == "completed" {
        if let Some(ga) = verification_agent {
            println!(
                "🔍 GROUNDING: Verifying success state for domain '{}' using agent '{}'...",
                macro_task_domain, ga.name
            );

            // SPECIAL CASE: If it's the standard grounding agent, it needs the state from paddle
            let mut state_context = String::new();
            if ga.name == "Grounding Agent" {
                let tools_list = crate::backend::get_tools(db).await.unwrap_or_default();
                if let Some(paddle_tool) = tools_list.iter().find(|t| t.name == "paddle_ui_automation") {
                    if let Ok(state_json) = crate::engine::tools::call_dynamic_tool(
                        paddle_tool,
                        serde_json::json!({"input": "get_state"}),
                    ).await {
                        state_context = state_json;
                    }
                }
            }

            let grounding_model = crate::backend::resolve_model_name(
                db,
                Some(&ga.model),
                "default_chat_model",
                "qwen2.5:7b-instruct",
            ).await;
            
            let ga_client = rig::agent::AgentBuilder::new(
                ollama_client.completion_model(&grounding_model),
            )
            .preamble(&ga.instructions)
            .build();

            let ga_prompt = format!(
                "GOAL: {}\nTASK: {}\n\nCONTEXT/UI STATE:\n{}\n\nHas the task been successfully completed? Respond with STATE_VERIFIED or STATE_MISMATCH: [reason]",
                query, step, state_context
            );

            if let Ok(Ok(ga_response)) = timeout(
                Duration::from_millis(model_timeout_ms),
                ga_client.prompt(ga_prompt),
            ).await {
                println!("👁️ GROUNDING EYE: {}", ga_response);
                if ga_response.contains("STATE_MISMATCH") {
                    supervisor_result = SupervisorExecutionResult {
                        status: "failed".to_string(),
                        summary: "Grounding verification failed.".to_string(),
                        evidence: Vec::new(),
                        artifacts: Vec::new(),
                        verified: false,
                        hide_evidence: false,
                        error: Some(ga_response.clone()),
                        raw_response: ga_response.clone(),
                    };
                }
            }
        }
    }

    Ok(supervisor_result)
}
