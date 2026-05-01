use serde_json::Value;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::response::{ChartPart, MessagePart};

pub fn parse_supervisor_result(raw: String, hide_evidence: bool, report_style: Option<String>) -> SupervisorExecutionResult {
    let mut merged_value = serde_json::Map::new();
    let mut summary = String::new();
    let mut json_found = false;
    
    // O(n) Single-pass Mixed Content Parser
    let mut brace_count = 0;
    let mut json_start = None;
    let chars: Vec<char> = raw.chars().collect();
    
    for (i, &c) in chars.iter().enumerate() {
        if c == '{' {
            if brace_count == 0 { json_start = Some(i); }
            brace_count += 1;
        } else if c == '}' {
            if brace_count > 0 {
                brace_count -= 1;
                if brace_count == 0 {
                    if let Some(start) = json_start {
                        let potential_raw: String = chars[start..=i].iter().collect();
                        // ROBUST REPAIR: Fix raw newlines/tabs in string literals before parsing
                        let potential = repair_json_literals(&potential_raw);
                        if let Ok(val) = serde_json::from_str::<Value>(&potential) {
                            json_found = true;
                            let target = val.get("final").unwrap_or(&val);
                            if let Some(obj) = target.as_object() {
                                // HEURISTIC: If it's a direct chart object at root
                                if obj.contains_key("chart_type") && (obj.contains_key("labels") || obj.contains_key("data")) {
                                    let arr = merged_value.entry("charts".to_string()).or_insert(Value::Array(Vec::new())).as_array_mut().unwrap();
                                    arr.push(target.clone());
                                }

                                for (k, v) in obj {
                                    match k.as_str() {
                                        "charts" | "evidence" | "evidences" | "artifacts" | "attachments" | "messages" => {
                                            let entry = merged_value.entry(k.clone()).or_insert(Value::Array(Vec::new()));
                                            if let Some(arr) = entry.as_array_mut() {
                                                if let Some(new_arr) = v.as_array() { arr.extend(new_arr.clone()); }
                                                else { arr.push(v.clone()); }
                                            }
                                        },
                                        "status" | "verified" => { merged_value.insert(k.clone(), v.clone()); },
                                        _ => {
                                            // Handle summaries inside JSON - we append them to our main summary
                                            if k == "summary" || k == "message" {
                                                let extracted_str = if let Some(s) = v.as_str() {
                                                    Some(s.to_owned())
                                                } else if let Some(arr) = v.as_array() {
                                                    // HEURISTIC: Flatten content arrays (common in Verifier output)
                                                    let mut flattened = String::new();
                                                    for item in arr {
                                                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                                            if !flattened.is_empty() { flattened.push_str(" "); }
                                                            flattened.push_str(text);
                                                            if let Some(src) = item.get("source").and_then(|s| s.as_str()) {
                                                                flattened.push_str(&format!(" ({})", src));
                                                            }
                                                        } else if let Some(s) = item.as_str() {
                                                            if !flattened.is_empty() { flattened.push_str(" "); }
                                                            flattened.push_str(s);
                                                        }
                                                    }
                                                    if flattened.is_empty() { Some(v.to_string()) } else { Some(flattened) }
                                                } else if let Some(obj) = v.as_object() {
                                                    obj.get("cleanedSummary")
                                                        .or(obj.get("cleaned_summary"))
                                                        .or(obj.get("text"))
                                                        .or(obj.get("content"))
                                                        .or(obj.get("message"))
                                                        .and_then(|val| {
                                                            if let Some(s) = val.as_str() { Some(s.to_owned()) }
                                                            else if val.is_array() || val.is_object() {
                                                                // Recursively handle nested content? 
                                                                // For now, just stringify if it's too deep, but Llama usually puts it in 'content' array.
                                                                None 
                                                            } else { None }
                                                        })
                                                        .or_else(|| Some(v.to_string()))
                                                } else {
                                                    None
                                                };

                                                if let Some(s) = extracted_str {
                                                    let trimmed_s = s.trim();
                                                    if !trimmed_s.is_empty() && !summary.contains(trimmed_s) {
                                                        if !summary.is_empty() { summary.push_str("\n\n"); }
                                                        summary.push_str(trimmed_s);
                                                    }
                                                }
                                            } else if !merged_value.contains_key(k) {
                                                merged_value.insert(k.clone(), v.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if brace_count == 0 {
            summary.push(c);
        }
    }

    // 2. Cleanup summary (strip markdown noise)
    summary = summary.replace("```json", "").replace("```", "").trim().to_string();

    // ARCHITECTURE V2: WILL-CHECK HEURISTIC
    // If the summary contains planning language, handle it as a stall even if status is 'completed'
    let plan_re = regex::Regex::new(r"(?i)\b(I will|I plan to|I need to|Next steps:|I'll perform|I am going to|I should|we could|will be searching|will try to)\b").unwrap();
    let contains_plan = plan_re.is_match(&summary);

    if json_found || !summary.is_empty() {
        let mut res = extract_result_from_value(Value::Object(merged_value), summary, raw.clone(), hide_evidence, report_style);
        
        // PHANTOM ARTIFACT DETECTION: If the model says it "generated" a file but none are in the list
        let mentions_generation = res.summary.to_lowercase().contains("generated") || res.summary.to_lowercase().contains("created") || res.summary.to_lowercase().contains("pdf") || res.summary.to_lowercase().contains("report");
        let has_no_artifacts = res.artifacts.is_empty();
        let is_phantom = mentions_generation && has_no_artifacts && (res.summary.to_lowercase().contains(".pdf") || res.summary.to_lowercase().contains("document") || res.summary.to_lowercase().contains("file"));

        if (contains_plan || is_phantom) && res.status == "completed" {
            println!("🔍 {} DETECTED in summary. Downgrading status to 'stalled'.", if contains_plan { "PLAN" } else { "PHANTOM ARTIFACT" });
            res.status = "stalled".to_string();
        }
        return res;
    }
    
    // Robust fall-back for failed parses: use summary if it was extracted, otherwise default to completed
    // ARCHITECTURE V2 HARDENING: Check for 'stalled' even in malformed JSON
    let is_stalled = contains_plan || raw.contains("\"status\": \"stalled\"") || raw.contains("\"status\":\"stalled\"");
    let final_status = if is_stalled { "stalled" } else { "completed" };
    
    let final_summary = if !summary.is_empty() { 
        summary 
    } else if is_stalled {
        "Worker process is stalled (planning without action). Triggering retry...".to_string()
    } else {
        "Task completed.".to_string() 
    };

    SupervisorExecutionResult {
        status: final_status.to_string(),
        summary: final_summary,
        messages: vec![],
        evidence: vec![],
        artifacts: vec![],
        charts: vec![],
        verified: is_stalled, // Mark as verified if we detected a stall
        hide_evidence: true,
        report_style,
        error: None,
        raw_response: raw,
        findings: std::collections::HashMap::new(),
    }
}

fn repair_json_literals(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_quotes = false;
    let mut escaped = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c = chars[i];
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

fn extract_result_from_value(value: Value, summary: String, raw: String, hide_evidence: bool, report_style: Option<String>) -> SupervisorExecutionResult {
    let target = value.get("final").unwrap_or(&value);
    
    let mut evidence = Vec::new();
    if let Some(e) = target.get("evidence").or(target.get("evidences")).and_then(|v| v.as_array()) {
        evidence = e.iter().filter_map(|v| {
            if let Some(s) = v.as_str() { Some(s.to_string()) }
            else if v.is_object() {
                Some(v.to_string())
            } else { None }
        }).collect();
    }
    
    // HEURISTIC: Capture all non-standard keys into 'findings'
    let mut findings = std::collections::HashMap::new();
    if let Some(obj) = target.as_object() {
        for (k, v) in obj {
            if !matches!(k.as_str(), "status" | "summary" | "evidence" | "evidences" | "artifacts" | "attachments" | "messages" | "charts" | "verified" | "final") {
                findings.insert(k.clone(), v.clone());
            }
        }
    }

    let mut final_summary = summary.replace("```json", "").replace("```", "").trim().to_string();
    if final_summary.is_empty() {
        if let Some(c) = target.get("charts").and_then(|v| v.as_array()).and_then(|a| a.first()) {
            let ctype = c.get("chart_type").and_then(|v| v.as_str()).unwrap_or("chart");
            let title = c.get("title").and_then(|v| v.as_str()).unwrap_or("Data visualization");
            final_summary = format!("Generated a {} chart: {}", ctype, title);
        } else if let Some(ctype) = target.get("chart_type").and_then(|v| v.as_str()) {
             let title = target.get("title").and_then(|v| v.as_str()).unwrap_or("Data visualization");
             final_summary = format!("Generated a {} chart: {}", ctype, title);
        } else {
            final_summary = "Task completed.".to_string();
        }
    }

    let mut artifacts = Vec::new();
    if let Some(a) = target.get("artifacts").or(target.get("attachments")).and_then(|v| v.as_array()) {
        artifacts = a.iter().filter_map(|v| {
            if let Some(s) = v.as_str() { Some(s.to_string()) }
            else if let Some(path) = v.get("path").and_then(|v| v.as_str()) { Some(path.to_string()) }
            else { None }
        }).collect();
    }

    let mut messages = Vec::new();
    if let Some(m) = target.get("messages") {
        if let Some(arr) = m.as_array() {
            messages = arr.iter().filter_map(|v| serde_json::from_value::<MessagePart>(v.clone()).ok()).collect();
        } else if let Ok(single) = serde_json::from_value::<MessagePart>(m.clone()) {
            messages.push(single);
        }
    }

    let mut charts = Vec::new();
    if let Some(c) = target.get("charts") {
        if let Some(arr) = c.as_array() {
            charts = arr.iter().filter_map(|v| serde_json::from_value::<ChartPart>(v.clone()).ok()).collect();
        } else if let Ok(single) = serde_json::from_value::<ChartPart>(c.clone()) {
            charts.push(single);
        }
    }

    let (verified, status) = if let Some(v) = target.get("status").or_else(|| target.get("verified")) {
        let s_val = v.as_str().map(|s| s.to_string()).unwrap_or_else(|| "unverified".to_string());
        
        let is_verified = if let Some(b) = v.as_bool() {
            b
        } else {
            let lower = s_val.to_lowercase();
            lower == "completed" || lower == "done" || lower == "success" || lower == "verified"
        };
        
        (is_verified, s_val)
    } else {
        (true, "completed".to_string())
    };

    SupervisorExecutionResult::new(status, final_summary.trim().to_string(), messages, evidence, artifacts, charts, verified, hide_evidence, report_style, raw, findings)
}

pub fn user_facing_runtime_error(details: &str) -> Option<String> {
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

pub fn parse_browser_state_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let mut evidence = Vec::new();
        if let Some(windows) = value.get("browser_windows").and_then(|item| item.as_array()) {
            for window in windows.iter().take(12) {
                let name = window.get("ProcessName").and_then(|item| item.as_str()).unwrap_or("browser");
                let title = window.get("MainWindowTitle").and_then(|item| item.as_str()).unwrap_or("Untitled");
                evidence.push(format!("{name}: {title}"));
            }
        }
        let summary = if evidence.is_empty() {
            "No open browser windows were detected.".to_string()
        } else {
            "Observed the current open browser windows on this machine.".to_string()
        };
        return (evidence, summary);
    }
    (vec!["Browser windows were inspected natively.".to_string()], "Observed the current browser state.".to_string())
}

pub fn parse_accessibility_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let mut evidence = Vec::new();
        let scope = value.get("search_scope").and_then(|item| item.as_str()).unwrap_or("active_window");
        let match_count = value.get("match_count").and_then(|item| item.as_u64()).unwrap_or(0);

        if let Some(focused) = value.get("focused_element").and_then(|item| item.as_object()) {
            let name = focused.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed element");
            let control_type = focused.get("control_type").and_then(|item| item.as_str()).unwrap_or("control");
            evidence.push(format!("Focused element: {} ({})", name, control_type));
        }

        if let Some(matches) = value.get("matches").and_then(|item| item.as_array()) {
            for matched in matches.iter().take(5) {
                let name = matched.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed element");
                let control_type = matched.get("control_type").and_then(|item| item.as_str()).unwrap_or("control");
                evidence.push(format!("Matched element: {} ({})", name, control_type));
            }
        }
        evidence.push(format!("Accessibility search scope: {}.", scope));
        let summary = if match_count > 0 {
            format!("Inspected accessibility state and found {} matching UI elements.", match_count)
        } else {
            "Inspected accessibility state for the current UI scope.".to_string()
        };
        return (evidence, summary);
    }
    (vec!["Accessibility state was inspected natively.".to_string()], "Inspected accessibility state for the current UI scope.".to_string())
}

pub fn parse_uia_interaction_payload(payload: &str) -> (Vec<String>, String, bool) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let action = value.get("action").and_then(|item| item.as_str()).unwrap_or("focus_element");
        let method = value.get("method").and_then(|item| item.as_str()).unwrap_or("uia");
        let scope = value.get("search_scope").and_then(|item| item.as_str()).unwrap_or("active_window");
        let element = value.get("element").and_then(|item| item.as_object());
        let verified = value.get("verified").and_then(|item| item.as_bool()).unwrap_or(false);
        let verification = value.get("verification").and_then(|item| item.as_str()).unwrap_or("not_checked");
        let name = element.and_then(|item| item.get("name")).and_then(|item| item.as_str()).unwrap_or("target element");
        let control_type = element.and_then(|item| item.get("control_type")).and_then(|item| item.as_str()).unwrap_or("control");

        let mut evidence = vec![
            format!("Matched UI element: {} ({})", name, control_type),
            format!("Interaction method: {}.", method),
            format!("UIA search scope: {}.", scope),
            format!("Verification: {}.", verification.replace('_', " ")),
        ];
        if let Some(text_length) = value.get("text_length").and_then(|item| item.as_u64()) {
            evidence.push(format!("Typed {} characters through the UI control.", text_length));
        }
        if let Some(post_focus) = value.get("post_action_focused_element").and_then(|item| item.as_object()) {
            let focus_name = post_focus.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed element");
            let focus_type = post_focus.get("control_type").and_then(|item| item.as_str()).unwrap_or("control");
            evidence.push(format!("Focused after action: {} ({})", focus_name, focus_type));
        }

        let summary = match action {
            "invoke_element" => format!("Invoked UI element '{}'.", name),
            "type_text" => format!("Entered text into UI element '{}'.", name),
            _ => format!("Focused UI element '{}'.", name),
        };
        return (evidence, summary, verified);
    }
    (vec!["A UI Automation element interaction completed natively.".to_string()], "Completed a native UI Automation interaction.".to_string(), false)
}

pub fn parse_uia_blocked_payload(payload: &str) -> (Vec<String>, String, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let action = value.get("action").and_then(|item| item.as_str()).unwrap_or("focus_element");
        let search_scope = value.get("search_scope").and_then(|item| item.as_str()).unwrap_or("active_window");
        let retry_hint = value.get("retry_hint").and_then(|item| item.as_str()).unwrap_or("Try a more specific control name or inspect accessibility first.");
        let app_hint = value.get("app_hint").and_then(|item| item.as_str());
        let selector = value.get("selector").and_then(|item| item.as_object());
        let selector_desc = selector.map(|selector| {
            let mut parts = Vec::new();
            for (key, label) in [
                ("name", "name"), ("automation_id", "automation id"), ("class_name", "class"),
                ("control_type", "control type"), ("selector", "selector"),
            ] {
                if let Some(value) = selector.get(key).and_then(|item| item.as_str()) {
                    if !value.trim().is_empty() { parts.push(format!("{}={}", label, value)); }
                }
            }
            parts.join(", ")
        }).filter(|item| !item.is_empty()).unwrap_or_else(|| "the requested selector".to_string());

        let mut evidence = vec![format!("UIA search scope: {}.", search_scope), format!("Requested selector: {}.", selector_desc)];
        if let Some(app_hint) = app_hint { evidence.push(format!("Preferred app window: {}.", app_hint)); }
        if let Some(active_window) = value.get("active_window").and_then(|item| item.as_object()) {
            let name = active_window.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed window");
            let class_name = active_window.get("class_name").and_then(|item| item.as_str()).unwrap_or("window");
            evidence.push(format!("Active window: {} ({})", name, class_name));
        }
        if let Some(focused) = value.get("focused_element").and_then(|item| item.as_object()) {
            let name = focused.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed element");
            let control_type = focused.get("control_type").and_then(|item| item.as_str()).unwrap_or("control");
            evidence.push(format!("Focused element: {} ({})", name, control_type));
        }
        if let Some(candidates) = value.get("candidates").and_then(|item| item.as_array()) {
            for candidate in candidates.iter().take(5) {
                let name = candidate.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed element");
                let control_type = candidate.get("control_type").and_then(|item| item.as_str()).unwrap_or("control");
                let automation_id = candidate.get("automation_id").and_then(|item| item.as_str()).unwrap_or("");
                if automation_id.is_empty() { evidence.push(format!("Closest candidate: {} ({})", name, control_type)); }
                else { evidence.push(format!("Closest candidate: {} ({}, automation id {})", name, control_type, automation_id)); }
            }
        }
        evidence.push(format!("Retry hint: {}.", retry_hint.trim_end_matches('.')));

        let summary = match action {
            "invoke_element" => "Blocked: Overseer could not find a matching UI element to invoke.".to_string(),
            "type_text" => "Blocked: Overseer could not find a matching UI field for text entry.".to_string(),
            _ => "Blocked: Overseer could not find a matching UI element to focus.".to_string(),
        };
        let error = format!("No UI Automation element matched the requested selector. {}", retry_hint);
        return (evidence, summary, error);
    }
    (vec!["No matching UI Automation element was found.".to_string()], "Blocked: Overseer could not find a matching UI element.".to_string(), "No matching UI Automation element was found.".to_string())
}

pub fn extract_uia_blocked_json(error: &str) -> Option<&str> {
    error.find("UIA_BLOCKED:").map(|index| error[index + "UIA_BLOCKED:".len()..].trim())
}

pub fn parse_file_search_payload(payload: &str) -> (Vec<String>, Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let mut evidence = Vec::new();
        let mut artifacts = Vec::new();
        if let Some(results) = value.get("results").and_then(|item| item.as_array()) {
            for result in results.iter().take(12) {
                let name = result.get("name").and_then(|item| item.as_str()).unwrap_or("unknown");
                let path = result.get("path").and_then(|item| item.as_str()).unwrap_or("");
                if !path.is_empty() {
                    evidence.push(format!("{name}: {path}"));
                    artifacts.push(path.to_string());
                }
            }
        }
        let summary = if let Some(first) = artifacts.first() {
            format!("Found matching file paths on this PC. Best match: {}", first)
        } else {
            "No matching file path was found on this PC.".to_string()
        };
        return (evidence, artifacts, summary);
    }
    (vec!["Local file search completed.".to_string()], Vec::new(), "Local file search completed.".to_string())
}

pub fn parse_document_writer_payload(payload: &str) -> (Vec<String>, Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let mut evidence = Vec::new();
        let mut artifacts = Vec::new();
        let message = value.get("message").and_then(|m| m.as_str()).unwrap_or("Document generation completed.");
        if let Some(items) = value.get("artifacts").and_then(|a| a.as_array()) {
            for item in items {
                let path = item.get("path").and_then(|p| p.as_str()).unwrap_or("");
                let label = item.get("label").and_then(|l| l.as_str()).unwrap_or("Export");
                if !path.is_empty() {
                    evidence.push(format!("{}: {}", label, path));
                    artifacts.push(path.to_string());
                }
            }
        }
        return (evidence, artifacts, message.to_string());
    }
    (vec!["Document generation completed.".to_string()], Vec::new(), "Document generation completed.".to_string())
}

pub fn parse_process_list_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let total = value.get("total_processes").and_then(|item| item.as_i64()).unwrap_or(0);
        let mut evidence = Vec::new();
        if let Some(processes) = value.get("background_processes").and_then(|item| item.as_array()) {
            for process in processes.iter().take(12) {
                let name = process.get("ProcessName").and_then(|item| item.as_str()).unwrap_or("unknown");
                let pid = process.get("Id").and_then(|item| item.as_i64()).unwrap_or_default();
                let cpu = process.get("CPU").and_then(|item| item.as_f64()).unwrap_or(0.0);
                evidence.push(format!("{name} (PID {pid}, CPU {cpu:.1})"));
            }
        }
        let summary = if total > 0 {
            format!("Observed {} running processes. Here are the current background tasks with the highest activity.", total)
        } else {
            "Observed the current running background tasks.".to_string()
        };
        return (evidence, summary);
    }
    (vec!["Current running processes were inspected natively.".to_string()], "Observed the current running background tasks.".to_string())
}

pub fn parse_screenshot_artifact_payload(payload: &str) -> (Vec<String>, Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let path = value.get("path").and_then(|item| item.as_str()).unwrap_or("screenshot artifact");
        let width = value.get("width").and_then(|item| item.as_i64());
        let height = value.get("height").and_then(|item| item.as_i64());
        let summary = match (width, height) {
            (Some(w), Some(h)) => format!("Captured a desktop screenshot at {}x{}.", w, h),
            _ => "Captured a desktop screenshot.".to_string(),
        };
        let mut evidence = vec![format!("Native screenshot artifact created at {}.", path)];
        if let (Some(w), Some(h)) = (width, height) { evidence.push(format!("Screenshot dimensions: {}x{}.", w, h)); }
        return (evidence, vec![path.to_string()], summary);
    }
    (vec!["Native screenshot artifact created.".to_string()], vec!["screenshot".to_string()], "Captured a desktop screenshot.".to_string())
}

pub fn parse_launch_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let app = value.get("app").and_then(|item| item.as_str()).unwrap_or("the requested app");
        return (vec![format!("Native app launch executed for {}.", app)], format!("Launched {}.", app));
    }
    (vec!["Native app launch executed.".to_string()], "Launched the requested app.".to_string())
}
