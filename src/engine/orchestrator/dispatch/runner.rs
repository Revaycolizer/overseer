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
    retry_message: Option<&str>,
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
            return Ok(SupervisorExecutionResult::completed(sm, vec![], ev, art, vec![], true, s.hide_evidence, s.report_style.clone(), res, std::collections::HashMap::new()));
        }
    }

    let agent = AgentBuilder::new(ollama_client.completion_model(&model))
        .preamble(s.instructions.as_deref().unwrap_or(""))
        .build();

    let tool_list = tools.iter().map(|t| format!("- {}: {}", t.name, t.description)).collect::<Vec<_>>().join("\n");
    let style = s.report_style.as_deref().unwrap_or("detailed");
    
    let retry_inject = match retry_message {
        Some(msg) => format!("\n\n⚠️ RETRY NOTICE: Your previous response was rejected. REASON: {}\nIMMEDIATE ACTION: You must call a tool NOW. Do NOT plan or describe future intent.", msg),
        None => "".to_string(),
    };

     let mut current_prompt = format!(
        "USER_ORIGINAL_QUERY: {}\n\
         CURRENT_TASK: {}\n\n\
         TOOLS:\n{}\n\n\
         RELEVANT_CONTEXT:\n{}\n\n\
         REPORTING_STYLE: {}\n\n\
         EXECUTION_HISTORY:\n{}\n\
         ═══ TRUTH CONTRACT (V3.1) ═══\n\
         You MUST:\n\
         - GENERATION RULE: You may ONLY generate sentences that can be directly mapped to a segment in the `STRUCTURED_STATE` using its `[E#]` ID.\n\
         - CITATION MANDATE: Every factual claim MUST conclude with a citation, e.g., \"Malaria is a mosquito-borne parasite [E4].\"\n\
         - DROPPING POLICY: If you lack evidence for a claim, do NOT mention it. Silently discard any unsupported intent.\n\
         - ABSTAIN PROTOCOL: If the evidence is insufficient to fulfill the task, you MUST return with `status: stalled` and `summary: ABSTAIN | REASON: INSUFFICIENT_EVIDENCE`.\n\
         - EVIDENCE ID PRESERVATION: Only use `[E#]` IDs provided in the current `STRUCTURED_STATE`. Do NOT fabricate new IDs.\n\
         - PHYSICAL REALITY: You are FORBIDDEN from returning `status: completed` if your task requires generating a file (PDF, DOCX, CSV, Image) UNLESS you have successfully called the appropriate tool (e.g., `document_writer`).\n\
         - Explicitly tag every significant claim with its source (e.g., [E1], [E2]).\n\n\
         INSTRUCTIONS:\n\
         You are an autonomous agent performing a specific task as part of a larger multi-step plan. \
         Execute the CURRENT_TASK using ONLY the provided evidence. Respond with a single JSON block. \
         If the task involves creating or exporting a document -> CALL `document_writer` NOW. Do not just report a path.{} \n\n\
         Your final response 'summary' MUST follow the requested REPORTING_STYLE. \
         The 'evidence' list should contain ALL specific data points, quotes, or links you discovered.",
        query, step, tool_list, memory_context, style, plan_history, retry_inject
    );

    println!(
        "Dispatching supervisor '{}' using model '{:?}'",
        s.name, s.model
    );
 
    let max_iterations = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'max_iterations'")
        .fetch_optional(&db.pool)
        .await
        .unwrap_or(None)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(10); // Fallback to 10 if missing or invalid

    let mut last_chart = None;
    let mut last_artifacts = Vec::new();
    let mut visited_urls = HashSet::new();
    let mut tools_executed_count = 0;
    let mut action_tool_executed = false;

    for iteration in 0..max_iterations {
        println!("Supervisor iteration {}/{}", iteration + 1, max_iterations);
        let response = agent.prompt(&current_prompt).await.map_err(|e| e.to_string())?;
        let trimmed = response.trim();
        println!("DEBUG: Supervisor '{}' (iteration {}) raw response: \n---\n{}\n---", s.name, iteration + 1, trimmed);

        // Robust tool extraction: find the OUTERMOST { } block
        let mut tool_json_str = None;
        let mut brace_count = 0;
        let mut json_start = None;
        let chars: Vec<char> = trimmed.chars().collect();
        
        for (i, &c) in chars.iter().enumerate() {
            if c == '{' {
                if brace_count == 0 { json_start = Some(i); }
                brace_count += 1;
            } else if c == '}' {
                if brace_count > 0 {
                    brace_count -= 1;
                    if brace_count == 0 {
                        if let Some(start) = json_start {
                            let potential: String = chars[start..=i].iter().collect();
                            // If it has a tool-calling field at root, treat as tool call
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&potential) {
                                let has_tool = json.get("tool").or(json.get("tool_call")).is_some();
                                if has_tool {
                                    tool_json_str = Some(potential);
                                    break;
                                }
                            } else {
                                // Try repair during detection for robustness
                                let repaired = repair_json_literals(&potential);
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&repaired) {
                                    if json.get("tool").or(json.get("tool_call")).is_some() {
                                        tool_json_str = Some(repaired);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if let Some(json_str) = tool_json_str {
            // --- Improved Extraction and Repair Logic ---
            let mut final_json: Option<serde_json::Value> = None;
            
            // 1. Try to find the JSON block if it's surrounded by markdown or text
            let target_str = if let Some(start) = json_str.find('{') {
                if let Some(end) = json_str.rfind('}') {
                    &json_str[start..=end]
                } else {
                    &json_str[start..]
                }
            } else {
                &json_str
            };

            // 2. Try standard parse
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(target_str) {
                final_json = Some(json);
            } else {
                // 3. Fallback: Heuristic repair of raw newlines or control characters in literals
                let repaired = repair_json_literals(target_str);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&repaired) {
                    println!("✨ [REPAIR] Successfully recovered JSON tool call by escaping raw literals.");
                    final_json = Some(json);
                }
            }

            if let Some(json) = final_json {
                // Support multiple synonyms for tool name and input
                let mut tname = json.get("tool").and_then(|v| v.as_str());
                let mut input = json.get("input").unwrap_or(&serde_json::Value::Null).clone();
                
                if tname.is_none() {
                    if let Some(tc) = json.get("tool_call") {
                        tname = tc.get("name").or(tc.get("function")).and_then(|v| v.as_str());
                        if let Some(ni) = tc.get("arguments").or(tc.get("input")) {
                            input = ni.clone();
                        }
                    }
                }
                
                if let Some(tname) = tname {
                    let tool = tools.iter().find(|t| t.name == tname).ok_or_else(|| format!("Tool {} missing", tname))?;
                        
                    println!("🛠️ EXECUTING TOOL: {} with input: {}", tname, input);
                    let output = crate::engine::tools::call_dynamic_tool(tool, input.clone()).await.map_err(|e| e.to_string())?;
                    
                    tools_executed_count += 1;
                    // Define action tools that count towards "physical reality"
                    let is_action = ["stealth_search", "web_search", "document_writer", "charting", "os_tool", "code_executor", "stealth_browse", "web_browse", "http_request", "web_extract"].iter().any(|&t| t == tname);
                    if is_action {
                        action_tool_executed = true;
                    }

                    // --- REALITY CHECK: Track URLs ---
                    match tname {
                        "stealth_search" | "web_search" => {
                            if let Some(start) = output.find("---SEARCH_RESULTS---") {
                                let json_input = &output[start + 20..].trim();
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_input) {
                                    if let Some(arr) = val.as_array() {
                                        for item in arr {
                                            if let Some(u) = item.get("url").and_then(|v| v.as_str()) {
                                                visited_urls.insert(u.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        "stealth_browse" | "web_browse" | "http_request" | "web_extract" => {
                            if let Some(u) = input.as_str() { visited_urls.insert(u.to_string()); }
                            else if let Some(u) = input.get("url").and_then(|v| v.as_str()) { visited_urls.insert(u.to_string()); }
                            
                            // Also add the actual final URL if returned
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&output) {
                                if let Some(u) = val.get("url").and_then(|v| v.as_str()) {
                                    visited_urls.insert(u.to_string());
                                }
                            }
                        },
                        _ => {}
                    }

                    if tname == "charting" { last_chart = Some(output.clone()); }
                    
                    // Sneak artifacts out of tool responses (like document_writer)
                    if let Ok(json_res) = serde_json::from_str::<serde_json::Value>(&output) {
                        if let Some(arts) = json_res.get("artifacts").and_then(|a| a.as_array()) {
                            for art in arts {
                                if let Some(path) = art.as_str() {
                                    if !last_artifacts.contains(&path.to_string()) {
                                        last_artifacts.push(path.to_string());
                                    }
                                } else if let Some(path_obj) = art.get("path").and_then(|p| p.as_str()) {
                                    if !last_artifacts.contains(&path_obj.to_string()) {
                                        last_artifacts.push(path_obj.to_string());
                                    }
                                }
                            }
                        }
                    }
                    let feedback_suffix = if tname == "document_writer" {
                        "DOCUMENT GENERATED SUCCESSFULLY. If this completes the user request, you MUST now provide your final response using your required JSON output format with status 'completed'."
                    } else {
                        "Next: Call another tool (JSON) or provide your final answer using the structured JSON schema."
                    };

                    current_prompt = format!(
                        "{}\n\n[RESULT for '{}': {}]\n\n{}",
                        current_prompt, tname, output, feedback_suffix
                    );
                    continue;
                }
            }
        }

        let mut res = parse_supervisor_result(trimmed.to_string(), s.hide_evidence, s.report_style.clone());
        
        // Data-driven chart injection
        if let Some(json_str) = last_chart {
            if let Ok(injected) = serde_json::from_str::<crate::engine::orchestrator::core::response::ChartPart>(&json_str) {
                if !res.charts.iter().any(|existing| existing.title == injected.title && existing.chart_type == injected.chart_type) {
                    res.charts.push(injected);
                }
            }
        }
        
        // ARTIFACT REALITY CHECK: Only allow artifacts that were actually produced by tools in this step.
        let original_artifact_count = res.artifacts.len();
        res.artifacts.retain(|a| last_artifacts.contains(a));
        
        // Ensure all actually created artifacts are included
        for art in &last_artifacts {
            if !res.artifacts.contains(art) {
                res.artifacts.push(art.clone());
            }
        }

        // EVIDENCE REALITY CHECK: Prune any evidence items that contain unvisited URLs.
        let original_evidence_count = res.evidence.len();
        res.evidence.retain(|e| {
            // If the evidence contains an http link, it MUST be in visited_urls
            if e.contains("http://") || e.contains("https://") {
                visited_urls.iter().any(|u| e.contains(u))
            } else {
                // If evidence contains citations like [E1] or [Source: ...] but no tool was called, suspicious!
                // However, we allow "logical" derived evidence as long as it's not claiming a fake URL source.
                true 
            }
        });

        // --- SILICON GUARDRAIL: PHYSICAL REALITY ENFORCEMENT ---
        // 1. If we forced a stall for hallucinated URLs or artifacts
        let artifacts_hallucinated = original_artifact_count > 0 && res.artifacts.is_empty() && last_artifacts.is_empty();
        let evidence_hallucinated = original_evidence_count > 0 && res.evidence.is_empty() && !visited_urls.is_empty();
        
        // 2. If the domain is an ACTION domain (search/office) but it finished without calling an action tool
        let is_action_domain = ["stealth", "web", "office", "computer", "desktop", "code_executor"].contains(&domain);
        let finished_without_action = res.status == "completed" && is_action_domain && !action_tool_executed;

        if res.status == "completed" && (artifacts_hallucinated || evidence_hallucinated || finished_without_action) {
            let reason = if artifacts_hallucinated { "Hallucinated artifact path" } else if evidence_hallucinated { "Hallucinated evidence URLs" } else { "Task completed without physical tool execution" };
            println!("⚠️ PHYSICAL REALITY FAILURE in domain '{}': {}. Forcing status to 'stalled'.", domain, reason);
            res.status = "stalled".to_string();
        }
        
        println!("DEBUG: Parsed result for '{}': summary='{}', evidence_count={}, artifact_count={}", s.name, res.summary, res.evidence.len(), res.artifacts.len());
        
        return Ok(res);
    }

    Err("Max iterations reached".into())
}

fn repair_json_literals(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_quotes = false;
    let mut escaped = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c = chars[i];
        
        // HEURISTIC: Catch unquoted [PLACEHOLDERS] and wrap them in quotes
        if !in_quotes && c == '[' && i + 1 < chars.len() && chars[i+1].is_alphabetic() {
            // Check if this is a [MESSY_TOKEN]
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == ' ') {
                j += 1;
            }
            if j < chars.len() && chars[j] == ']' {
                // If it's something like [MISSING], check if the character before was a ":" or "," or " "
                // If so, it likely needs quotes.
                out.push('"');
                out.push_str(&chars[i..=j].iter().collect::<String>());
                out.push('"');
                i = j + 1;
                continue;
            }
        }

        match c {
            '"' if !escaped => {
                in_quotes = !in_quotes;
                out.push(c);
            }
            '\\' if !escaped => {
                escaped = true;
                out.push(c);
            }
            '\n' | '\r' if in_quotes => {
                out.push_str("\\n");
                escaped = false;
            }
            '\t' if in_quotes => {
                out.push_str("\\t");
                escaped = false;
            }
            _ => {
                out.push(c);
                escaped = false;
            }
        }
        i += 1;
    }
    out
}
