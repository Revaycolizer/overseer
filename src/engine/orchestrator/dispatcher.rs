use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::completion::{Message, Prompt};
use rig::providers::ollama;
use serde::{Deserialize, Serialize};

use crate::engine::orchestrator::OverseerAgent;
use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;

fn user_facing_runtime_error(details: &str) -> Option<String> {
    let lower = details.to_lowercase();

    let ollama_related = lower.contains("localhost:11434")
        || lower.contains("/api/chat")
        || lower.contains("/api/generate")
        || lower.contains("/api/embed")
        || lower.contains("ollama");

    let connection_related = lower.contains("error sending request")
        || lower.contains("connection refused")
        || lower.contains("failed to connect")
        || lower.contains("http client error")
        || lower.contains("tcp connect error")
        || lower.contains("dns error");

    if ollama_related || connection_related {
        return Some(
            "Model backend unavailable. Overseer could not reach Ollama on `http://localhost:11434`.\n\n\
             Start Ollama with `ollama serve`, verify it with `ollama list`, then retry the task."
                .to_string(),
        );
    }

    None
}

async fn log_event(
    db: &crate::db::Database,
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
            db,
            run_id,
            current_sequence,
            event_type,
            status,
            domain,
            step_text,
            tool_name,
            model,
            message,
            details,
        )
        .await;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorExecutionResult {
    pub status: String,
    pub summary: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub verified: bool,
    pub error: Option<String>,
    pub raw_response: String,
}

impl SupervisorExecutionResult {
    fn completed(
        summary: String,
        evidence: Vec<String>,
        artifacts: Vec<String>,
        verified: bool,
        raw_response: String,
    ) -> Self {
        Self {
            status: "completed".to_string(),
            summary,
            evidence,
            artifacts,
            verified,
            error: None,
            raw_response,
        }
    }

    fn failed(summary: String, error: String, raw_response: String) -> Self {
        Self {
            status: "failed".to_string(),
            summary,
            evidence: Vec::new(),
            artifacts: Vec::new(),
            verified: false,
            error: Some(error),
            raw_response,
        }
    }

    pub fn display_text(&self) -> String {
        let mut parts = vec![self.summary.clone()];

        if !self.evidence.is_empty() {
            parts.push(format!("Evidence:\n- {}", self.evidence.join("\n- ")));
        }

        if !self.artifacts.is_empty() {
            parts.push(format!("Artifacts:\n- {}", self.artifacts.join("\n- ")));
        }

        if !self.verified {
            parts.push(
                "Verification: unverified. The supervisor did not provide enough evidence to fully confirm this result."
                    .to_string(),
            );
        }

        if let Some(error) = &self.error {
            parts.push(format!("Error: {}", error));
        }

        parts.join("\n\n")
    }
}

pub async fn dispatch_domain(
    db: &crate::db::Database,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    agent_registry: &[AgentInfo],
    domain: &str,
    step: &str,
    query: &str,
    _history: Vec<Message>,
    _memories: &str,
    plan_history: &str,
    memory_context: &str,
    _macro_task_description: &str,
    _perception_tool: &StructuredPerceptionTool,
    ollama_client: &ollama::Client,
    model_timeout_ms: u64,
    _browser_pid: &mut Option<i32>,
) -> Result<SupervisorExecutionResult, String> {
    let supervisor = sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT id, name, description, instructions, model, domain, version, code, bypass_synthesizer FROM supervisors WHERE domain = ?"
    )
    .bind(domain)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| format!("Database error resolving supervisor: {}", e))?;

    let _linked_agents = if let Some(ref s) = supervisor {
        crate::backend::get_supervisor_agents(db, s.id)
            .await
            .map_err(|e| format!("Error fetching supervisor agents: {}", e))?
    } else {
        vec![]
    };

    let get_agent = |domain_name: &str| {
        agent_registry
            .iter()
            .find(|a| a.domain == domain_name)
            .cloned()
            .unwrap_or_else(|| AgentInfo {
                id: 0,
                name: domain_name.to_string(),
                description: String::new(),
                tools: Vec::new(),
                instructions: String::new(),
                model: "qwen2.5:7b-instruct".to_string(),
                domain: domain_name.to_string(),
                version: "1.0.0".to_string(),
                code: None,
                source_files: Vec::new(),
            })
    };

    let overseer_info = get_agent("overseer");
    let _overseer = OverseerAgent::new(
        ollama_client,
        &overseer_info.model,
        &overseer_info.instructions,
    );

    println!("Dispatching dynamic supervisor for domain '{}'", domain);
    let s = supervisor
        .clone()
        .ok_or_else(|| format!("No supervisor found in DB for domain '{}'.", domain))?;
    println!(
        "Dispatching supervisor '{}' using model '{:?}'",
        s.name, s.model
    );

    let tools = crate::backend::get_supervisor_tools(db, s.id)
        .await
        .unwrap_or_default();

    let agent = AgentBuilder::new(
        ollama_client.completion_model(s.model.as_deref().unwrap_or("qwen2.5:7b-instruct")),
    )
    .preamble(s.instructions.as_deref().unwrap_or(""))
    .build();

    let tool_descriptions = if tools.is_empty() {
        String::new()
    } else {
        let list = tools
            .iter()
            .map(|t| format!("- {} [{}]: {}", t.name, t.safety_level, t.description))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "AVAILABLE TOOLS:\n{}\n\n\
             To use a tool, respond ONLY with JSON: {{\"tool\": \"tool_name\", \"input\": {{...}}}}\n\
             If you are returning a FINAL answer instead of a tool call, respond ONLY with JSON in this shape:\n\
             {{\"status\":\"completed|unverified|failed\",\"summary\":\"short result\",\"evidence\":[\"proof 1\"],\"artifacts\":[\"file/path/or/chart\"],\"verified\":true|false,\"error\":\"optional error\"}}\n\
             OPTIONAL CHARTING: Use the charting tool ONLY if a visualization is explicitly requested or if you are comparing 3 or more data points.\n\
             SAFETY: Prefer lower-risk tools first. Treat tools marked 'dangerous' or 'high' as explicit-intent-only.\n\
             When you call charting, its result (starting with ---CHART_SVG---) MUST be your COMPLETE final response.\n\
             Never claim completion without evidence. If you are uncertain, use status 'unverified'. If something failed, use status 'failed'.\n\
             Do NOT add any text, markdown, or summary before or after a chart marker.\n\n",
            list
        )
    };

    let has_charting_tool = tools.iter().any(|t| t.name == "charting");
    let clean_context = sanitize_context(memory_context);
    let clean_plan = sanitize_context(plan_history);

    let initial_prompt = format!(
        "GOAL: {}\n\
         TASK: {}\n\n\
         {}CONTEXT:\n{}\n\n\
         PROGRESS:\n{}\n\n\
         COMMAND: Execute the task using tools if necessary. Final answers must use the structured JSON result schema.",
        query, step, tool_descriptions, clean_context, clean_plan
    );

    log_event(
        db,
        run_id,
        event_sequence,
        "supervisor_selected",
        "running",
        Some(domain),
        Some(step),
        None,
        s.model.as_deref(),
        Some("Supervisor selected for micro-step."),
        None,
    )
    .await;

    println!("Prompt text:\n{}", initial_prompt);
    println!("Supervisor preamble:\n{}", s.instructions.as_deref().unwrap_or(""));

    let max_iterations = crate::backend::get_setting_i64(db, "max_iterations", 10)
        .await
        .max(1) as usize;
    let tool_timeout_ms = crate::backend::get_setting_i64(db, "tool_timeout_ms", 45_000)
        .await
        .max(1) as u64;

    let mut current_prompt = initial_prompt;
    let mut final_response: Option<SupervisorExecutionResult> = None;
    let mut manual_svg_retries = 0;
    const MAX_SVG_RETRIES: usize = 2;

    for iteration in 0..max_iterations {
        println!("Supervisor iteration {}/{}", iteration + 1, max_iterations);

        let iteration_message = format!("Supervisor iteration {}", iteration + 1);
        log_event(
            db,
            run_id,
            event_sequence,
            "supervisor_iteration",
            "running",
            Some(domain),
            Some(step),
            None,
            s.model.as_deref(),
            Some(&iteration_message),
            None,
        )
        .await;

        let response = tokio::time::timeout(
            std::time::Duration::from_millis(model_timeout_ms),
            agent.prompt(&current_prompt),
        )
        .await
        .map_err(|_| {
            format!(
                "Dynamic supervisor '{}' timed out after {} ms.",
                s.name, model_timeout_ms
            )
        })?
        .map_err(|e| {
            eprintln!("Supervisor model call failed: {}", e);
            if let Some(friendly_error) = user_facing_runtime_error(&e.to_string()) {
                friendly_error
            } else {
                format!("Dynamic supervisor error for domain '{}': {}", domain, e)
            }
        })?;

        let trimmed = response.trim();
        println!(
            "Supervisor raw model response (first 500 chars): {}",
            &trimmed[..trimmed.len().min(500)]
        );

        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(tool_name) = json_val.get("tool").and_then(|v| v.as_str()) {
                let input_val = json_val.get("input").unwrap_or(&serde_json::Value::Null);

                let input_json = if input_val.is_string() {
                    let raw_input = input_val.as_str().unwrap_or_default();
                    match serde_json::from_str::<serde_json::Value>(raw_input) {
                        Ok(parsed) => parsed,
                        Err(_) => serde_json::json!({ "input": raw_input }),
                    }
                } else {
                    input_val.clone()
                };

                let tool_info = tools.iter().find(|t| t.name == tool_name);

                if let Some(info) = tool_info {
                    let requires_intent = info
                        .required_intent
                        .as_ref()
                        .map(|v| !v.trim().is_empty())
                        .unwrap_or(false);
                    let is_high_risk = matches!(info.safety_level.as_str(), "high" | "dangerous");
                    if requires_intent || is_high_risk {
                        let keywords: Vec<&str> = info
                            .required_intent
                            .as_deref()
                            .unwrap_or("")
                            .split(',')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect();
                        let user_query_lower = query.to_lowercase();
                        let explicitly_requested = if keywords.is_empty() {
                            !is_high_risk
                        } else {
                            keywords.iter().any(|&k| user_query_lower.contains(k))
                        };

                        if !explicitly_requested {
                            let reason = if keywords.is_empty() {
                                format!(
                                    "Tool '{}' is marked '{}' and requires explicit user intent.",
                                    tool_name, info.safety_level
                                )
                            } else {
                                format!(
                                    "Tool '{}' requires intent keywords [{}] before it can run.",
                                    tool_name,
                                    info.required_intent.as_deref().unwrap_or("")
                                )
                            };
                            log_event(
                                db,
                                run_id,
                                event_sequence,
                                "tool_blocked",
                                "blocked",
                                Some(domain),
                                Some(step),
                                Some(tool_name),
                                s.model.as_deref(),
                                Some(&reason),
                                Some(&input_json.to_string()),
                            )
                            .await;
                            current_prompt = format!(
                                "{}\n\n[SYSTEM OVERRIDE: {}]\n\nNext: Provide your answer using the structured JSON result schema.",
                                current_prompt, reason
                            );
                            continue;
                        }
                    }

                    log_event(
                        db,
                        run_id,
                        event_sequence,
                        "tool_requested",
                        "running",
                        Some(domain),
                        Some(step),
                        Some(tool_name),
                        s.model.as_deref(),
                        Some("Supervisor requested tool execution."),
                        Some(&input_json.to_string()),
                    )
                    .await;
                }

                let tool_result = match tool_info {
                    Some(info) => {
                        match tokio::time::timeout(
                            std::time::Duration::from_millis(tool_timeout_ms),
                            crate::engine::tools::call_dynamic_tool(info, input_json.clone()),
                        )
                        .await
                        {
                            Ok(Ok(result)) => result,
                            Ok(Err(e)) => format!("Tool error: {}", e),
                            Err(_) => format!(
                                "Tool error: '{}' timed out after {} ms.",
                                tool_name, tool_timeout_ms
                            ),
                        }
                    }
                    None => format!("Tool '{}' not found.", tool_name),
                };

                log_event(
                    db,
                    run_id,
                    event_sequence,
                    "tool_completed",
                    "completed",
                    Some(domain),
                    Some(step),
                    Some(tool_name),
                    s.model.as_deref(),
                    Some("Tool finished execution."),
                    Some(&tool_result),
                )
                .await;

                if tool_name == "charting"
                    && (tool_result.starts_with("---CHART_SVG---")
                        || tool_result.starts_with("---CHART_HTML---"))
                {
                    final_response = Some(SupervisorExecutionResult::completed(
                        "Generated a chart artifact.".to_string(),
                        vec!["Charting tool completed successfully.".to_string()],
                        vec!["chart".to_string()],
                        true,
                        tool_result,
                    ));
                    break;
                }

                current_prompt = format!(
                    "{}\n\n[RESULT for '{}' ({}):\n{}]\n\nNext: Call another tool (JSON) or provide your final answer using the structured JSON result schema.",
                    current_prompt, tool_name, input_val, tool_result
                );
                continue;
            }
        }

        if domain == "chat" && has_charting_tool && contains_manual_visualization(&response) {
            manual_svg_retries += 1;
            println!(
                "Manual SVG or HTML detected in supervisor output (retry {}/{}).",
                manual_svg_retries, MAX_SVG_RETRIES
            );

            if manual_svg_retries <= MAX_SVG_RETRIES {
                current_prompt = format!(
                    "{}\n\n[SYSTEM OVERRIDE: Your previous response contained manual SVG/HTML code. \
                    This is FORBIDDEN. You MUST call the charting tool instead. \
                    Do NOT write any SVG, HTML, or visualization code. \
                    Extract the data values from your previous response and call the charting tool now: \
                    {{\"tool\": \"charting\", \"input\": {{\"chart_type\": \"pie|bar|line\", \
                    \"title\": \"your title\", \"labels\": [\"2024\", \"2025\", \"2026\"], \
                    \"data\": [value1, value2, value3]}}}}]\n\n\
                    Call the charting tool NOW with the correct data.",
                    current_prompt
                );
                continue;
            }

            final_response = Some(SupervisorExecutionResult::failed(
                "Chart generation failed.".to_string(),
                "Model wrote manual SVG instead of using the charting tool.".to_string(),
                response.clone(),
            ));
            break;
        }

        let processed = post_process_for_domain(domain, response);
        final_response = Some(parse_supervisor_result(processed));
        break;
    }

    Ok(final_response.unwrap_or_else(|| {
        SupervisorExecutionResult::failed(
            "Supervisor stopped without a final result.".to_string(),
            "Max tool iterations reached without a final response.".to_string(),
            String::new(),
        )
    }))
}

fn contains_manual_visualization(response: &str) -> bool {
    let lower = response.to_lowercase();
    if lower.contains("<svg") && !lower.contains("---chart_svg---") {
        return true;
    }
    if lower.contains("```html") || lower.contains("```svg") || lower.contains("```xml") {
        return true;
    }
    if lower.contains("<!-- svg") || lower.contains("<!-- chart") {
        return true;
    }
    false
}

fn parse_supervisor_result(response: String) -> SupervisorExecutionResult {
    let cleaned = crate::engine::utils::clean_response(response.clone());

    if cleaned.starts_with("---CHART_SVG---") || cleaned.starts_with("---CHART_HTML---") {
        return SupervisorExecutionResult::completed(
            "Generated a chart artifact.".to_string(),
            vec!["Charting tool returned a chart artifact.".to_string()],
            vec!["chart".to_string()],
            true,
            cleaned,
        );
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&cleaned) {
        if value.is_object() {
            let status = value
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("completed")
                .to_string();
            let summary = value
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or(cleaned.as_str())
                .to_string();
            let evidence = value
                .get("evidence")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let artifacts = value
                .get("artifacts")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let verified = value
                .get("verified")
                .and_then(|v| v.as_bool())
                .unwrap_or(!evidence.is_empty());
            let error = value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            return SupervisorExecutionResult {
                status,
                summary,
                evidence,
                artifacts,
                verified,
                error,
                raw_response: cleaned,
            };
        }
    }

    SupervisorExecutionResult {
        status: "unverified".to_string(),
        summary: cleaned.clone(),
        evidence: Vec::new(),
        artifacts: Vec::new(),
        verified: false,
        error: None,
        raw_response: cleaned,
    }
}

fn post_process_for_domain(domain: &str, response: String) -> String {
    match domain {
        "stealth" => clean_stealth_output(response),
        _ => post_process_agent_response(response),
    }
}

fn clean_stealth_output(response: String) -> String {
    response
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#')
                || trimmed.starts_with("```")
                || trimmed == "---"
                || trimmed == "***"
            {
                return None;
            }
            let cleaned = trimmed
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim_start_matches("â€¢ ")
                .replace("**", "")
                .replace("__", "")
                .replace('*', "")
                .replace('_', "");
            if cleaned.trim().is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn post_process_agent_response(response: String) -> String {
    if let Some(pos) = response
        .find("---CHART_SVG---")
        .or_else(|| response.find("---CHART_HTML---"))
    {
        let before = response[..pos].trim().to_string();
        if before.is_empty() {
            return response[pos..].to_string();
        }
        return before;
    }
    response
}

fn sanitize_context(context: &str) -> String {
    let svg_tags = [
        "<rect ",
        "<path ",
        "<polyline ",
        "<text ",
        "<svg",
        "</svg>",
        "<style",
        "<defs",
        "<clipPath",
        "data:image/svg",
    ];

    context
        .lines()
        .filter(|line| {
            let lower = line.trim().to_lowercase();
            !svg_tags.iter().any(|tag| lower.contains(tag))
                && !lower.starts_with("```svg")
                && !lower.starts_with("```html")
                && lower != "```"
                && !lower.contains("![")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
