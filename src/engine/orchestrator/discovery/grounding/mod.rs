pub mod heuristics;

use crate::db::Database;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::utils::discovery::AgentInfo;
use heuristics::*;
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
    let supervisor =
        sqlx::query_as::<_, crate::db::Supervisor>("SELECT * FROM supervisors WHERE domain = ?")
            .bind(macro_task_domain)
            .fetch_optional(&db.pool)
            .await
            .map_err(|e| format!("DB error loading supervisor: {}", e))?;

    let verification_agent = if let Some(ref s) = supervisor {
        if let Some(agent_id) = s.verification_agent_id {
            all_agents.iter().find(|a| a.id == agent_id).cloned()
        } else if ["web", "desktop", "computer", "app"].contains(&macro_task_domain) {
            all_agents
                .iter()
                .find(|a| a.name == "Grounding Agent")
                .cloned()
        } else {
            None
        }
    } else {
        None
    };

    if supervisor_result.status == "completed" {
        if let Some(ga) = verification_agent {
            let state_context = if ga.name == "Grounding Agent" {
                gather_native_state(db).await
            } else {
                String::new()
            };
            if quick_verify(query, step, &supervisor_result, &state_context) {
                supervisor_result.verified = true;
                return Ok(supervisor_result);
            }

            let ga_prompt = format!("GOAL: {}\nTASK: {}\nSTATE: {}\nSUMMARY: {}\nEVIDENCE: {}\nHas the task been successfully completed? Respond with STATE_VERIFIED or STATE_MISMATCH: [reason]",
                query, step, state_context, supervisor_result.summary, supervisor_result.evidence.join("\n"));

            let grounding_model = crate::backend::resolve_model_name(
                db,
                Some(&ga.model),
                "default_chat_model",
                "qwen2.5:7b-instruct",
            )
            .await;
            let ga_client =
                rig::agent::AgentBuilder::new(ollama_client.completion_model(&grounding_model))
                    .preamble(&ga.instructions)
                    .build();

            if let Ok(Ok(ga_response)) = timeout(
                Duration::from_millis(model_timeout_ms),
                ga_client.prompt(ga_prompt),
            )
            .await
            {
                if ga_response.contains("STATE_MISMATCH") {
                    return Ok(SupervisorExecutionResult::failed(
                        "Grounding verification failed.".to_string(),
                        ga_response.clone(),
                        vec![],
                        ga_response,
                        None,
                    ));
                }
            }
        }
    }
    Ok(supervisor_result)
}

fn quick_verify(query: &str, step: &str, res: &SupervisorExecutionResult, ctx: &str) -> bool {
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    (combined.contains("screenshot")
        && has_native_evidence(res, ctx, looks_like_screenshot_artifact))
        || (combined.contains("process")
            && has_native_evidence(res, ctx, looks_like_process_observation))
        || (combined.contains("browser")
            && has_native_evidence(res, ctx, looks_like_browser_observation))
        || (combined.contains("file")
            && has_native_evidence(res, ctx, looks_like_file_search_observation))
}

async fn gather_native_state(db: &Database) -> String {
    let tools = crate::backend::get_tools(db).await.unwrap_or_default();
    if let Some(t) = tools.iter().find(|t| t.name == "vision_analysis") {
        if let Ok(s) = crate::engine::tools::call_dynamic_tool(
            t,
            serde_json::json!({"input": "live desktop session"}),
        )
        .await
        {
            return s;
        }
    }
    if let Some(t) = tools.iter().find(|t| t.name == "os_tool") {
        if let Ok(s) =
            crate::engine::tools::call_dynamic_tool(t, serde_json::json!({"action": "get_state"}))
                .await
        {
            return s;
        }
    }
    String::new()
}
