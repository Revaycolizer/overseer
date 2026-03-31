use crate::db::Database;
use crate::engine::orchestrator::result::SupervisorExecutionResult;
use crate::engine::utils::discovery::AgentInfo;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::ollama;
use std::time::Duration;
use tokio::time::timeout;

fn looks_like_screenshot_artifact(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains(".png")
        || lower.contains("screenshot")
        || lower.contains("\"action\":\"screenshot\"")
}

fn parse_json_value(value: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(value).ok()
}

fn value_has_nonempty_array(json: &serde_json::Value, key: &str) -> bool {
    json.get(key)
        .and_then(|value| value.as_array())
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

fn value_has_truthy_count(json: &serde_json::Value, key: &str) -> bool {
    json.get(key)
        .and_then(|value| value.as_i64())
        .map(|count| count > 0)
        .unwrap_or(false)
}

fn looks_like_process_observation(value: &str) -> bool {
    let lower = value.to_lowercase();
    if lower.contains("\"action\":\"list_processes\"")
        || lower.contains("\"background_processes\"")
        || lower.contains("\"top_processes\"")
        || lower.contains("current background tasks")
    {
        return true;
    }

    parse_json_value(value)
        .map(|json| {
            json.get("action")
                .and_then(|value| value.as_str())
                .map(|action| action == "list_processes")
                .unwrap_or(false)
                && (value_has_nonempty_array(&json, "background_processes")
                    || value_has_nonempty_array(&json, "top_processes")
                    || value_has_truthy_count(&json, "background_count")
                    || value_has_truthy_count(&json, "total_processes"))
        })
        .unwrap_or(false)
}

fn looks_like_browser_observation(value: &str) -> bool {
    let lower = value.to_lowercase();
    if lower.contains("\"action\":\"browser_state\"") || lower.contains("\"browser_windows\"") {
        return true;
    }

    parse_json_value(value)
        .map(|json| {
            json.get("action")
                .and_then(|value| value.as_str())
                .map(|action| action == "browser_state")
                .unwrap_or(false)
                && (value_has_nonempty_array(&json, "browser_windows")
                    || json
                        .get("browser_count")
                        .and_then(|value| value.as_i64())
                        .map(|count| count >= 0)
                        .unwrap_or(false))
        })
        .unwrap_or(false)
}

fn looks_like_file_search_observation(value: &str) -> bool {
    let lower = value.to_lowercase();
    if lower.contains("\"action\":\"find_files\"") || lower.contains("\"results\"") {
        return true;
    }

    parse_json_value(value)
        .map(|json| {
            json.get("action")
                .and_then(|value| value.as_str())
                .map(|action| action == "find_files")
                .unwrap_or(false)
                && value_has_nonempty_array(&json, "results")
        })
        .unwrap_or(false)
}

fn has_native_evidence(
    supervisor_result: &SupervisorExecutionResult,
    state_context: &str,
    predicate: impl Fn(&str) -> bool,
) -> bool {
    supervisor_result
        .artifacts
        .iter()
        .any(|item| predicate(item))
        || supervisor_result
            .evidence
            .iter()
            .any(|item| predicate(item))
        || predicate(&supervisor_result.raw_response)
        || predicate(&supervisor_result.summary)
        || predicate(state_context)
}

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
    let supervisor =
        sqlx::query_as::<_, crate::db::Supervisor>("SELECT * FROM supervisors WHERE domain = ?")
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
                all_agents
                    .iter()
                    .find(|a| a.name == "Grounding Agent")
                    .cloned()
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

            // SPECIAL CASE: If it's the standard grounding agent, gather current state from
            // the native computer tools instead of relying on the old paddle path.
            let mut state_context = String::new();
            if ga.name == "Grounding Agent" {
                let tools_list = crate::backend::get_tools(db).await.unwrap_or_default();
                if let Some(vision_tool) = tools_list.iter().find(|t| t.name == "vision_analysis") {
                    if let Ok(state_json) = crate::engine::tools::call_dynamic_tool(
                        vision_tool,
                        serde_json::json!({"input": "live desktop session"}),
                    )
                    .await
                    {
                        state_context = state_json;
                    }
                }
                if state_context.trim().is_empty() {
                    if let Some(os_tool) = tools_list.iter().find(|t| t.name == "os_tool") {
                        if let Ok(state_json) = crate::engine::tools::call_dynamic_tool(
                            os_tool,
                            serde_json::json!({"action": "get_state"}),
                        )
                        .await
                        {
                            state_context = state_json;
                        }
                    }
                }
            }

            let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());

            let screenshot_request = combined.contains("screenshot")
                || combined.contains("screen capture")
                || combined.contains("capture screen")
                || combined.contains("capture my desktop");
            let process_request = (combined.contains("background")
                || combined.contains("running")
                || combined.contains("process")
                || combined.contains("task"))
                && (combined.contains("currently")
                    || combined.contains("current")
                    || combined.contains("what")
                    || combined.contains("which")
                    || combined.contains("tell me")
                    || combined.contains("list")
                    || combined.contains("show"));
            let browser_request = (combined.contains("browser")
                || combined.contains("tab")
                || combined.contains("page"))
                && (combined.contains("current")
                    || combined.contains("which")
                    || combined.contains("what")
                    || combined.contains("open")
                    || combined.contains("state"));
            let file_search_request = (combined.contains("find")
                || combined.contains("locate")
                || combined.contains("search")
                || combined.contains("where")
                || combined.contains("path"))
                && (combined.contains("file")
                    || combined.contains("resume")
                    || combined.contains(".pdf")
                    || combined.contains(".doc")
                    || combined.contains(".docx")
                    || combined.contains("path"));

            let has_screenshot_artifact = has_native_evidence(
                &supervisor_result,
                &state_context,
                looks_like_screenshot_artifact,
            );
            let has_process_observation = has_native_evidence(
                &supervisor_result,
                &state_context,
                looks_like_process_observation,
            );
            let has_browser_observation = has_native_evidence(
                &supervisor_result,
                &state_context,
                looks_like_browser_observation,
            );
            let has_file_search_observation = has_native_evidence(
                &supervisor_result,
                &state_context,
                looks_like_file_search_observation,
            );

            if (screenshot_request && has_screenshot_artifact)
                || (process_request && has_process_observation)
                || (browser_request && has_browser_observation)
                || (file_search_request && has_file_search_observation)
            {
                supervisor_result.verified = true;
                return Ok(supervisor_result);
            }

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

            let evidence_context = if supervisor_result.evidence.is_empty() {
                "No explicit evidence supplied.".to_string()
            } else {
                supervisor_result.evidence.join("\n")
            };
            let artifact_context = if supervisor_result.artifacts.is_empty() {
                "No artifacts supplied.".to_string()
            } else {
                supervisor_result.artifacts.join("\n")
            };

            let ga_prompt = format!(
                "GOAL: {}\nTASK: {}\n\nCONTEXT/UI STATE:\n{}\n\nEXECUTION SUMMARY:\n{}\n\nEVIDENCE:\n{}\n\nARTIFACTS:\n{}\n\nRAW RESPONSE:\n{}\n\nGrounding rules:\n- Native screenshot artifacts (.png or screenshot JSON) count as successful completion for screenshot tasks.\n- Do not require opening a UI application like Snipping Tool unless the task explicitly asked for it.\n- If the task is completed by native tool evidence or artifacts, respond with STATE_VERIFIED.\n\nHas the task been successfully completed? Respond with STATE_VERIFIED or STATE_MISMATCH: [reason]",
                query,
                step,
                state_context,
                supervisor_result.summary,
                evidence_context,
                artifact_context,
                supervisor_result.raw_response
            );

            if let Ok(Ok(ga_response)) = timeout(
                Duration::from_millis(model_timeout_ms),
                ga_client.prompt(ga_prompt),
            )
            .await
            {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_observation_recognizes_native_payload() {
        let payload = r#"{"action":"list_processes","total_processes":42,"background_count":12,"background_processes":[{"ProcessName":"ollama"}]}"#;
        assert!(looks_like_process_observation(payload));
    }

    #[test]
    fn browser_observation_recognizes_native_payload() {
        let payload = r#"{"action":"browser_state","browser_count":1,"browser_windows":[{"ProcessName":"chrome","MainWindowTitle":"Docs"}]}"#;
        assert!(looks_like_browser_observation(payload));
    }

    #[test]
    fn file_search_observation_requires_results() {
        let payload = r#"{"action":"find_files","results":[{"path":"C:\\Users\\Administrator\\Desktop\\overseer\\src\\state.rs"}]}"#;
        assert!(looks_like_file_search_observation(payload));
    }
}
