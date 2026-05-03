use rig::agent::AgentBuilder;
use rig::completion::{Message, Prompt};
use rig::providers::ollama;
use crate::db::Database;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::state::StructuredState;
use crate::engine::utils::discovery::AgentInfo;
use super::prompt_builder::build_dispatch_prompt;
use super::json_utils::repair_json_literals;
use super::tool_exec::execute_tool;
use super::reality::enforce_physical_reality;
use rig::client::CompletionClient;
use std::collections::HashSet;

pub async fn dispatch_domain(
    db: &Database,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    _registry: &[AgentInfo],
    domain: &str,
    step: &str,
    query: &str,
    _history: Vec<Message>,
    plan_history: &str,
    state: &StructuredState,
    ollama_client: &ollama::Client,
    _timeout: u64,
    retry_msg: Option<&str>,
) -> Result<SupervisorExecutionResult, String> {
    let supervisor = sqlx::query_as::<_, crate::db::Supervisor>("SELECT * FROM supervisors WHERE domain = ?").bind(domain).fetch_one(&db.pool).await.map_err(|e| e.to_string())?;
    let model = crate::backend::resolve_model_name(db, supervisor.model.as_deref(), "default_chat_model", "qwen2.5:7b-instruct").await;
    let mut tools = crate::backend::get_supervisor_tools(db, supervisor.id).await.unwrap_or_default();

    // SAFETY NET: If the office domain has no tools (broken supervisor_agents link),
    // fetch document_writer directly so the LLM always has it available.
    let is_doc_domain = ["office", "text_writer"].contains(&domain);
    if is_doc_domain && !tools.iter().any(|t| t.name == "document_writer") {
        if let Ok(all_tools) = crate::backend::get_tools(db).await {
            if let Some(dw) = all_tools.into_iter().find(|t| t.name == "document_writer") {
                tools.push(dw);
            }
        }
    }

    
    let agent = AgentBuilder::new(ollama_client.completion_model(&model)).preamble(supervisor.instructions.as_deref().unwrap_or("")).build();
    let tool_list = tools.iter().map(|t| format!("- {}: {}", t.name, t.description)).collect::<Vec<_>>().join("\n");
    
    let mut current_prompt = build_dispatch_prompt(query, step, &tool_list, &state.to_prompt_string(), supervisor.report_style.as_deref().unwrap_or("detailed"), plan_history, retry_msg);
    let mut visited_urls = state.get_known_urls();
    let mut last_artifacts = Vec::new();
    let mut action_tool_executed = false;

    for iteration in 0..10 {
        let response = agent.prompt(&current_prompt).await.map_err(|e| e.to_string())?;
        let repaired = repair_json_literals(response.trim());
        let json: serde_json::Value = match serde_json::from_str(&repaired) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let mut res = crate::engine::orchestrator::dispatch::parsers::parse_supervisor_result(repaired, supervisor.hide_evidence, supervisor.report_style.clone());

        if let Some(tname) = json.get("tool").and_then(|v| v.as_str()) {
            let input = json.get("input").cloned().unwrap_or(serde_json::Value::Null);
            match execute_tool(tname, input, &tools, state).await {
                Ok((output, is_action, expanded_input)) => {
                    if is_action { action_tool_executed = true; }
                    current_prompt = format!("{}\n\n[RESULT for '{}': {}]", current_prompt, tname, output);
                    
                    // Robust artifact extraction
                    let mut art_found = false;
                    if let Ok(res_json) = serde_json::from_str::<serde_json::Value>(&output) {
                        if let Some(arts) = res_json.get("artifacts").and_then(|a| a.as_array()) {
                            for a in arts { 
                                if let Some(p) = a.get("path").and_then(|v| v.as_str()) { 
                                    last_artifacts.push(p.to_string()); 
                                    art_found = true;
                                } 
                            }
                        }
                    } 
                    
                    if !art_found && output.contains("assets/exports/") {
                        if let Some(start) = output.find("assets/exports/") {
                            let end = output[start..].find(|c: char| c == '"' || c == ' ' || c == ']' || c == '}').unwrap_or(output.len() - start);
                            let path = &output[start..start+end];
                            last_artifacts.push(path.to_string());
                        }
                    }


                    // Terminal Exit for Artifact Tools (especially document_writer)
                    if (tname == "document_writer" && !output.contains("ERROR:")) || !last_artifacts.is_empty() {
                        res.artifacts = last_artifacts.clone();
                        res.status = "completed".to_string();
                        if !last_artifacts.is_empty() {
                            res.summary = format!("Generated document: {}", last_artifacts.join(", "));
                            res.messages.clear();
                        }
                        return Ok(res);
                    }
                    continue;
                }
                Err(stalled) => return Ok(stalled),
            }
        }

        // OFFICE DOMAIN FALLBACK: ... (existing code) ...
        if is_doc_domain && !action_tool_executed {
            if let Some(dw_tool) = tools.iter().find(|t| t.name == "document_writer") {
                let content = format!("{}\n\n{}", plan_history, state.to_prompt_string());
                let auto_input = serde_json::json!({"format": "pdf", "content": content});
                println!("🔧 OFFICE FALLBACK: Auto-invoking document_writer with state context.");
                match execute_tool("document_writer", auto_input, &tools, state).await {
                    Ok((output, _, _)) => {
                        action_tool_executed = true;
                        if let Ok(res_json) = serde_json::from_str::<serde_json::Value>(&output) {
                            if let Some(arts) = res_json.get("artifacts").and_then(|a| a.as_array()) {
                                for a in arts { if let Some(p) = a.get("path").and_then(|v| v.as_str()) { last_artifacts.push(p.to_string()); } }
                            }
                        }
                        res.artifacts = last_artifacts.clone();
                        res.status = "completed".to_string();
                        if !last_artifacts.is_empty() {
                            res.summary = format!("Generated document: {}", last_artifacts.join(", "));
                            res.messages.clear();
                        }
                        return Ok(res);
                    }
                    Err(stalled) => return Ok(stalled),
                }
            }
        }

        enforce_physical_reality(&mut res, &visited_urls, &last_artifacts, action_tool_executed, domain);
        return Ok(res);

    }
    Err("Max iterations reached".into())
}
