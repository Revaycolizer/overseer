use std::collections::HashSet;
use rig::agent::AgentBuilder;
use rig::completion::{Message, Prompt};
use rig::client::CompletionClient;
use rig::providers::ollama;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::utils::formatter::format_supervisor_result;
use crate::engine::utils::discovery::{AgentInfo};
use crate::db::Database;
use super::parsers::*;
use super::native::*;
use super::tools::*;

async fn log_event(
    db: &Database,
    run_id: Option<i64>,
    sequence: &mut i64,
    event_type: &str,
    status: &str,
    domain: Option<&str>,
    step_text: Option<&str>,
    tool_name: Option<&str>,
    model: Option<&str>,
    message: Option<&str>,
    details: Option<&str>,
) {
    if let Some(run_id) = run_id {
        let current_sequence = *sequence;
        *sequence += 1;
        let _ = crate::backend::log_execution_event(
            db, run_id, current_sequence, event_type, status,
            domain, step_text, tool_name, model, message, details,
        ).await;
    }
}

pub async fn dispatch_domain(
    db: &Database,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    _agent_registry: &[AgentInfo],
    domain: &str,
    step: &str,
    query: &str,
    _history: Vec<Message>,
    plan_history: &str,
    memory_context: &str,
    ollama_client: &ollama::Client,
    model_timeout_ms: u64,
) -> Result<SupervisorExecutionResult, String> {
    let supervisor = sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT * FROM supervisors WHERE domain = ?"
    )
    .bind(domain)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let s = supervisor.ok_or_else(|| format!("Supervisor '{}' not found", domain))?;
    let model = crate::backend::resolve_model_name(db, s.model.as_deref(), "default_chat_model", "qwen2.5:7b-instruct").await;
    let tools = crate::backend::get_supervisor_tools(db, s.id).await.unwrap_or_default();
    
    // Fast Path: Screenshot
    if is_native_screenshot_request(query, step, domain) {
        if let Some(t) = tools.iter().find(|t| t.name == "os_tool") {
            let res = crate::engine::tools::call_dynamic_tool(t, serde_json::json!({"action":"screenshot"})).await.map_err(|e| e.to_string())?;
            let (ev, art, sm) = parse_screenshot_artifact_payload(&res);
            return Ok(SupervisorExecutionResult::completed(sm, ev, art, vec![], true, s.hide_evidence, res));
        }
    }

    let agent = AgentBuilder::new(ollama_client.completion_model(&model))
        .preamble(s.instructions.as_deref().unwrap_or(""))
        .build();

    let tool_list = tools.iter().map(|t| format!("- {}: {}", t.name, t.description)).collect::<Vec<_>>().join("\n");
    let mut current_prompt = format!(
        "GOAL: {}\nTASK: {}\nTOOLS:\n{}\nCONTEXT:\n{}\nPLAN:\n{}\n\nRespond with tool JSON or FINAL goal JSON.",
        query, step, tool_list, memory_context, plan_history
    );

    let mut last_chart = None;
    for _ in 0..10 {
        let response = agent.prompt(&current_prompt).await.map_err(|e| e.to_string())?;
        let trimmed = response.trim();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(tname) = json.get("tool").and_then(|v| v.as_str()) {
                let tool = tools.iter().find(|t| t.name == tname).ok_or_else(|| format!("Tool {} missing", tname))?;
                let input = json.get("input").unwrap_or(&serde_json::Value::Null).clone();
                let output = crate::engine::tools::call_dynamic_tool(tool, input).await.map_err(|e| e.to_string())?;
                if tname == "charting" { last_chart = Some(output.clone()); }
                current_prompt = format!("{}\n\n[RESULT for '{}': {}]", current_prompt, tname, output);
                continue;
            }
        }

        let mut res = parse_supervisor_result(trimmed.to_string(), s.hide_evidence);
        if let Some(chart) = last_chart {
            if res.raw_response.contains("---CHART_SVG---") {
                res.raw_response = format!("{}\n\n{}", res.raw_response, chart);
            }
        }
        return Ok(res);
    }

    Err("Max iterations reached".into())
}
