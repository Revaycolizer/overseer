use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::completion::{Message, Prompt};
use rig::providers::ollama;

use crate::engine::orchestrator::OverseerAgent;
use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;

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
) -> Result<String, String> {
    let supervisor = sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT id, name, description, instructions, model, domain, version, code, bypass_synthesizer FROM supervisors WHERE domain = ?"
    )
    .bind(domain)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| format!("Database error resolving supervisor: {}", e))?;

    let _linked_agents = if let Some(ref s) = supervisor {
        crate::backend::get_supervisor_agents(db, s.id).await
            .map_err(|e| format!("Error fetching supervisor agents: {}", e))?
    } else {
        vec![]
    };

    let get_agent = |domain_name: &str| {
        agent_registry.iter().find(|a| a.domain == domain_name)
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
    let _overseer = OverseerAgent::new(ollama_client, &overseer_info.model, &overseer_info.instructions);

    println!("🤖 DISPATCH: Dynamic supervisor for domain '{}'", domain);
    let s = supervisor.clone().ok_or_else(|| format!("No supervisor found in DB for domain '{}'.", domain))?;
    println!("🤖 DISPATCH: Dynamic supervisor '{}' using model '{:?}'", s.name, s.model);

    let tools = crate::backend::get_supervisor_tools(db, s.id)
        .await
        .unwrap_or_default();

    let agent = AgentBuilder::new(
        ollama_client.completion_model(s.model.as_deref().unwrap_or("qwen2.5:7b-instruct"))
    )
    .preamble(s.instructions.as_deref().unwrap_or(""))
    .build();

    let tool_descriptions = if tools.is_empty() {
        String::new()
    } else {
        let list = tools.iter()
            .map(|t| format!("- {} [{}]: {}", t.name, t.safety_level, t.description))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "AVAILABLE TOOLS:\n{}\n\n\
             To use a tool, respond ONLY with JSON: {{\"tool\": \"tool_name\", \"input\": {{...}}}}\n\
             OPTIONAL CHARTING: Use the charting tool ONLY if a visualization is explicitly requested or if you are comparing 3 or more data points.\n\
             SAFETY: Prefer lower-risk tools first. Treat tools marked 'dangerous' or 'high' as explicit-intent-only.\n\
             When you call charting, its result (starting with ---CHART_SVG---) MUST be your COMPLETE final response.\n\
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
         COMMAND: Execute the task using tools if necessary. Provide your final findings.",
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

    println!("📝 PROMPT TEXT:\n{}", initial_prompt);
    println!("📜 PREAMBLE:\n{}", s.instructions.as_deref().unwrap_or(""));

    let max_iterations = crate::backend::get_setting_i64(db, "max_iterations", 10).await.max(1) as usize;
    let tool_timeout_ms = crate::backend::get_setting_i64(db, "tool_timeout_ms", 45_000).await.max(1) as u64;

    let mut current_prompt = initial_prompt;
    let mut final_response = String::new();
    let mut manual_svg_retries = 0;
    const MAX_SVG_RETRIES: usize = 2;

    for iteration in 0..max_iterations {
        println!("🔄 AGENT ITERATION {}/{}", iteration + 1, max_iterations);

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
        .map_err(|_| format!("Dynamic supervisor '{}' timed out after {} ms.", s.name, model_timeout_ms))?
        .map_err(|e| {
            eprintln!("❌ ERROR calling dynamic agent: {}", e);
            format!("Dynamic supervisor error for domain '{}': {}", domain, e)
        })?;

        let trimmed = response.trim();
        println!("🔍 RAW MODEL RESPONSE (first 500 chars): {}", &trimmed[..trimmed.len().min(500)]);

        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(tool_name) = json_val.get("tool").and_then(|v| v.as_str()) {
                let input_val = json_val.get("input").unwrap_or(&serde_json::Value::Null);
                println!(
                    "🛠️ MODEL CALLED TOOL: '{}' | raw input type: {}",
                    tool_name,
                    if input_val.is_string() {
                        "string (will try to parse as JSON)"
                    } else if input_val.is_object() {
                        "object"
                    } else {
                        "other"
                    }
                );

                let input_json = if input_val.is_string() {
                    let s = input_val.as_str().unwrap();
                    println!("📋 RAW INPUT STRING: {}", s);
                    match serde_json::from_str::<serde_json::Value>(s) {
                        Ok(parsed) => {
                            println!("✅ Input string parsed as JSON: {}", parsed);
                            parsed
                        }
                        Err(e) => {
                            println!("⚠️ Input string is NOT valid JSON ({}), wrapping as {{input: s}}", e);
                            serde_json::json!({"input": s})
                        }
                    }
                } else {
                    println!("📋 INPUT OBJECT: {}", input_val);
                    input_val.clone()
                };

                let tool_info = tools.iter().find(|t| t.name == tool_name);

                if let Some(info) = tool_info {
                    let requires_intent = info.required_intent.as_ref().map(|v| !v.trim().is_empty()).unwrap_or(false);
                    let is_high_risk = matches!(info.safety_level.as_str(), "high" | "dangerous");
                    if requires_intent || is_high_risk {
                        let keywords: Vec<&str> = info.required_intent
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
                                format!("Tool '{}' is marked '{}' and requires explicit user intent.", tool_name, info.safety_level)
                            } else {
                                format!(
                                    "Tool '{}' requires intent keywords [{}] before it can run.",
                                    tool_name,
                                    info.required_intent.as_deref().unwrap_or("")
                                )
                            };
                            println!("🚫 DYNAMIC INTENT GUARDRAIL: {}", reason);
                            let blocked_details = input_json.to_string();
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
                                Some(&blocked_details),
                            )
                            .await;
                            let next_instruction = format!("Next: Provide your answer in PLAIN TEXT. {}", reason);
                            current_prompt = format!(
                                "{}\n\n[SYSTEM OVERRIDE: {}]\n\n{}",
                                current_prompt,
                                reason,
                                next_instruction
                            );
                            continue;
                        }
                    }

                    let tool_details = input_json.to_string();
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
                        Some(&tool_details),
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
                            Err(_) => format!("Tool error: '{}' timed out after {} ms.", tool_name, tool_timeout_ms),
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

                if tool_name == "charting" {
                    println!("📦 CHARTING TOOL RESULT (first 200): {}", &tool_result[..tool_result.len().min(200)]);
                    println!(
                        "📦 CHARTING RESULT STARTS WITH MARKER: {}",
                        tool_result.starts_with("---CHART_SVG---") || tool_result.starts_with("---CHART_HTML---")
                    );
                } else {
                    println!("📦 TOOL RESULT: {}", &tool_result[..tool_result.len().min(300)]);
                }

                if tool_name == "charting"
                    && (tool_result.starts_with("---CHART_SVG---") || tool_result.starts_with("---CHART_HTML---"))
                {
                    println!("✅ Chart generated — returning directly without model passthrough");
                    final_response = tool_result;
                    break;
                }

                if tool_name == "charting" {
                    println!(
                        "❌ CHARTING TOOL DID NOT RETURN A CHART MARKER — result was: {}",
                        &tool_result[..tool_result.len().min(500)]
                    );
                }

                current_prompt = format!(
                    "{}\n\n[RESULT for '{}' ({}):\n{}]\n\nNext: Call another tool (JSON) or provide your final answer.",
                    current_prompt, tool_name, input_val, tool_result
                );
                continue;
            }
        } else {
            println!("🔍 NOT A TOOL CALL — treating as final answer candidate");
        }

        if domain == "chat" && has_charting_tool && contains_manual_visualization(&response) {
            manual_svg_retries += 1;
            println!("⚠️ Agent wrote manual SVG/HTML (retry {}/{}). Forcing charting tool call.", manual_svg_retries, MAX_SVG_RETRIES);

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
            } else {
                println!("❌ Max SVG retries exceeded. Agent refuses to use charting tool.");
                final_response = "Chart generation failed — model wrote manual SVG instead of using the charting tool. Please try again.".to_string();
                break;
            }
        }

        println!("✅ FINAL RESPONSE received at iteration {}", iteration + 1);
        final_response = post_process_for_domain(domain, response);
        break;
    }

    if final_response.is_empty() {
        final_response = "Max tool iterations reached without a final response.".to_string();
    }

    Ok(final_response)
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
            if trimmed.starts_with('#') || trimmed.starts_with("```") || trimmed == "---" || trimmed == "***" {
                return None;
            }
            let cleaned = trimmed
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim_start_matches("• ")
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
    if let Some(pos) = response.find("---CHART_SVG---").or_else(|| response.find("---CHART_HTML---")) {
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
