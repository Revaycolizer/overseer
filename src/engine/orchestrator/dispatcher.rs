use std::collections::HashSet;

use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::completion::{Message, Prompt};
use rig::providers::ollama;
use serde::{Deserialize, Serialize};

use super::record_event;
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

#[derive(Debug, Clone)]
struct ToolGateDecision {
    allowed: bool,
    reason: Option<String>,
    action_summary: Option<String>,
    action_tool: bool,
}

fn is_computer_observation_tool(tool_name: &str) -> bool {
    matches!(tool_name, "vision_analysis")
}

fn is_computer_action_tool(domain: &str, tool_name: &str, safety_level: &str) -> bool {
    if tool_name == "os_tool" {
        return true;
    }

    if matches!(safety_level, "high" | "dangerous") && matches!(domain, "computer" | "desktop") {
        return true;
    }

    matches!(domain, "computer" | "desktop") && !is_computer_observation_tool(tool_name)
}

fn summarize_computer_action(tool_name: &str, input_json: &serde_json::Value) -> String {
    if let Some(input) = input_json.get("input").and_then(|value| value.as_str()) {
        return format!("{} -> {}", tool_name, input.trim());
    }

    if let Some(action) = input_json.get("action").and_then(|value| value.as_str()) {
        if let Some(app) = input_json.get("app").and_then(|value| value.as_str()) {
            return format!("{} -> {} ({})", tool_name, action.trim(), app.trim());
        }
        return format!("{} -> {}", tool_name, action.trim());
    }

    if let Some(target) = input_json
        .get("target")
        .and_then(|value| value.as_str())
        .or_else(|| input_json.get("window").and_then(|value| value.as_str()))
        .or_else(|| input_json.get("title").and_then(|value| value.as_str()))
    {
        return format!("{} -> {}", tool_name, target.trim());
    }

    format!("{} -> {}", tool_name, input_json)
}

fn computer_strategy_hint(
    query: &str,
    step: &str,
    tools: &[crate::engine::utils::discovery::ToolInfo],
) -> Option<String> {
    let lower = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    if !(lower.contains("computer")
        || lower.contains("desktop")
        || lower.contains("window")
        || lower.contains("screen"))
    {
        return None;
    }

    let has_os_tool = tools.iter().any(|tool| tool.name == "os_tool");
    let has_vision_tool = tools.iter().any(|tool| tool.name == "vision_analysis");
    if !has_os_tool && !has_vision_tool {
        return None;
    }

    let mut guidance = Vec::new();
    if lower.contains("screenshot")
        || lower.contains("screen capture")
        || lower.contains("capture screen")
    {
        if has_os_tool {
            guidance.push("For screenshot tasks, call `os_tool` with JSON like {\"action\":\"screenshot\"}. Do not open Snipping Tool or any other UI unless the user explicitly asked for that app.");
        }
        if has_vision_tool {
            guidance.push("After taking a screenshot, use `vision_analysis` only if you need to confirm the live desktop state or reference the screenshot artifact.");
        }
        guidance
            .push("A native screenshot artifact counts as completion for a screenshot request.");
    }

    if lower.contains("active window")
        || lower.contains("which window")
        || lower.contains("what window")
        || lower.contains("current window")
        || lower.contains("foreground window")
        || lower.contains("observe")
        || lower.contains("inspect")
    {
        if has_vision_tool {
            guidance.push("For desktop observation, call `vision_analysis` first to inspect the live desktop session.");
        }
        if has_os_tool {
            guidance.push("Use `os_tool` with action `get_state` or `list_windows` to gather evidence about the active window and visible windows.");
        }
        guidance.push("Do not answer from memory. Observation tasks require tool evidence before you summarize anything.");
    }

    if lower.contains("background task")
        || lower.contains("background tasks")
        || lower.contains("running process")
        || lower.contains("running processes")
        || lower.contains("what's running")
        || lower.contains("whats running")
    {
        if has_os_tool {
            guidance.push("Use `os_tool` with action `list_processes` to inspect the machine's currently running processes and background tasks.");
        }
        guidance.push("Do not explain Windows settings. Report the actual currently running processes from tool output.");
    }

    if lower.contains("resume")
        || lower.contains("file path")
        || lower.contains("find file")
        || lower.contains("locate file")
        || ((lower.contains("find") || lower.contains("locate") || lower.contains("where is"))
            && (lower.contains("file") || lower.contains("path")))
    {
        if has_os_tool {
            guidance.push("Use `os_tool` with action `find_files` to search the local machine and return exact matching file paths. If the user wants the file contents, follow up with `read_file`. If they want it opened, follow up with `open_file`.");
        }
        guidance.push("Do not open File Explorer unless the user explicitly asked you to. Return the real file path from native search results.");
    }

    if lower.contains("browser")
        || lower.contains("current tab")
        || lower.contains("current page")
        || lower.contains("which browser")
    {
        if has_os_tool {
            guidance.push("Use `os_tool` with action `browser_state` to inspect open browser windows and titles on the local machine.");
        }
        guidance.push("Do not guess the user's browser state. Report the actual open browser windows from tool output.");
    }

    if guidance.is_empty() {
        None
    } else {
        Some(guidance.join(" "))
    }
}

fn extract_allowlist_target(input_json: &serde_json::Value) -> Option<String> {
    [
        "app",
        "application",
        "window",
        "title",
        "process",
        "target",
        "url",
    ]
    .into_iter()
    .find_map(|key| {
        input_json
            .get(key)
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn session_elapsed_timed_out(started_at: Option<&str>, timeout_ms: i64) -> bool {
    let Some(started_at) = started_at else {
        return false;
    };

    if timeout_ms <= 0 {
        return false;
    }

    chrono::DateTime::parse_from_rfc3339(started_at)
        .ok()
        .map(|started| {
            let elapsed =
                chrono::Utc::now().signed_duration_since(started.with_timezone(&chrono::Utc));
            elapsed.num_milliseconds() >= timeout_ms
        })
        .unwrap_or(false)
}

async fn evaluate_computer_tool_gate(
    db: &crate::db::Database,
    domain: &str,
    tool_name: &str,
    safety_level: &str,
    input_json: &serde_json::Value,
) -> ToolGateDecision {
    let action_tool = is_computer_action_tool(domain, tool_name, safety_level);
    let touches_computer = matches!(domain, "computer" | "desktop")
        || action_tool
        || is_computer_observation_tool(tool_name);

    if !touches_computer {
        return ToolGateDecision {
            allowed: true,
            reason: None,
            action_summary: None,
            action_tool: false,
        };
    }

    let state = crate::backend::get_computer_supervision_state(db).await;
    let action_summary = summarize_computer_action(tool_name, input_json);

    if state.session_state == "emergency_stopped" {
        return ToolGateDecision {
            allowed: false,
            reason: Some(
                "Computer supervision is in emergency stop. Reset the session in Computer Control before any desktop action can continue."
                    .to_string(),
            ),
            action_summary: Some(action_summary),
            action_tool,
        };
    }

    if state.session_state == "paused" {
        return ToolGateDecision {
            allowed: false,
            reason: Some(
                "Computer supervision is paused. Resume the session in Computer Control before continuing."
                    .to_string(),
            ),
            action_summary: Some(action_summary),
            action_tool,
        };
    }

    if action_tool && state.session_state == "idle" {
        return ToolGateDecision {
            allowed: false,
            reason: Some(
                "No supervised computer session is active. Start an approval or trusted session from Computer Control before running desktop actions."
                    .to_string(),
            ),
            action_summary: Some(action_summary),
            action_tool,
        };
    }

    if action_tool
        && (state.supervision_mode == "observe_only" || state.session_state == "observing")
    {
        return ToolGateDecision {
            allowed: false,
            reason: Some(
                "Computer supervision is currently observe-only. Propose the next action, but do not execute it."
                    .to_string(),
            ),
            action_summary: Some(action_summary),
            action_tool,
        };
    }

    if action_tool && state.max_actions > 0 && state.action_count >= state.max_actions {
        return ToolGateDecision {
            allowed: false,
            reason: Some(format!(
                "Computer supervision hit the per-session action limit ({}). Start a fresh session or raise the limit before continuing.",
                state.max_actions
            )),
            action_summary: Some(action_summary),
            action_tool,
        };
    }

    if action_tool
        && session_elapsed_timed_out(state.session_started_at.as_deref(), state.timeout_ms)
    {
        return ToolGateDecision {
            allowed: false,
            reason: Some(format!(
                "Computer supervision timed out after {} ms. Start a fresh session before continuing.",
                state.timeout_ms
            )),
            action_summary: Some(action_summary),
            action_tool,
        };
    }

    if action_tool && !state.allowlist.is_empty() {
        if let Some(target) = extract_allowlist_target(input_json) {
            let target_lower = target.to_lowercase();
            let allowed = state
                .allowlist
                .iter()
                .any(|entry| target_lower.contains(&entry.to_lowercase()));
            if !allowed {
                return ToolGateDecision {
                    allowed: false,
                    reason: Some(format!(
                        "The requested target '{}' does not match the current computer allowlist. Narrow the action or update the allowlist in Computer Control.",
                        target
                    )),
                    action_summary: Some(action_summary),
                    action_tool,
                };
            }
        }
    }

    let requires_approval = action_tool
        && (state.supervision_mode == "approval_required"
            || state.session_state == "approval_session");

    if requires_approval {
        let pending_matches =
            state.pending_tool.trim().is_empty() || state.pending_tool == tool_name;

        if state.approval_state == "approved_once" && pending_matches {
            return ToolGateDecision {
                allowed: true,
                reason: None,
                action_summary: Some(action_summary),
                action_tool,
            };
        }

        let _ =
            crate::backend::stage_computer_approval_request(db, domain, tool_name, &action_summary)
                .await;

        return ToolGateDecision {
            allowed: false,
            reason: Some(format!(
                "Approval required before '{}' can run. Review the pending action in Computer Control, approve it, then continue.",
                tool_name
            )),
            action_summary: Some(action_summary),
            action_tool,
        };
    }

    ToolGateDecision {
        allowed: true,
        reason: None,
        action_summary: Some(action_summary),
        action_tool,
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
    #[serde(default)]
    pub hide_evidence: bool,
    pub error: Option<String>,
    pub raw_response: String,
}

impl SupervisorExecutionResult {
    fn completed(
        summary: String,
        evidence: Vec<String>,
        artifacts: Vec<String>,
        verified: bool,
        hide_evidence: bool,
        raw_response: String,
    ) -> Self {
        Self {
            status: "completed".to_string(),
            summary,
            evidence,
            artifacts,
            verified,
            hide_evidence,
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
            hide_evidence: false,
            error: Some(error),
            raw_response,
        }
    }

    pub fn display_text(&self) -> String {
        fn normalize_summary(summary: &str) -> String {
            summary
                .lines()
                .map(|line| {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with('#') {
                        trimmed.trim_start_matches('#').trim().to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        }

        fn extract_artifact_path(item: &str) -> Option<String> {
            if item.to_lowercase().contains(".png")
                || item.to_lowercase().contains(".jpg")
                || item.to_lowercase().contains(".jpeg")
                || item.to_lowercase().contains(".webp")
            {
                return Some(item.trim().to_string());
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(item) {
                if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
                    return Some(path.to_string());
                }
            }
            None
        }

        fn screenshot_dimensions(evidence: &[String], raw_response: &str) -> Option<String> {
            for item in evidence {
                let lower = item.to_lowercase();
                if lower.contains("screenshot dimensions:") {
                    return Some(item.trim().to_string());
                }
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw_response) {
                if let Some(width) = value.get("width").and_then(|v| v.as_i64()) {
                    if let Some(height) = value.get("height").and_then(|v| v.as_i64()) {
                        return Some(format!("Dimensions: {}x{}.", width, height));
                    }
                }
            }
            None
        }

        fn is_html_noise(value: &str) -> bool {
            let lower = value.to_lowercase();
            lower.contains("<img ") || lower.contains("```html") || lower.contains("<html")
        }

        let mut final_summary = normalize_summary(&self.summary);

        let screenshot_artifact = self
            .artifacts
            .iter()
            .find_map(|item| extract_artifact_path(item))
            .or_else(|| {
                self.evidence
                    .iter()
                    .find_map(|item| extract_artifact_path(item))
            });

        if let Some(path) = screenshot_artifact {
            let mut parts = vec![if final_summary.is_empty() {
                "Desktop screenshot captured successfully.".to_string()
            } else {
                final_summary
            }];
            if let Some(dimensions) = screenshot_dimensions(&self.evidence, &self.raw_response) {
                parts.push(dimensions);
            }
            if std::path::Path::new(&path).exists() {
                parts.push("Screenshot attached below.".to_string());
            }
            if let Some(error) = &self.error {
                parts.push(format!("Error: {}", error));
            }
            if std::path::Path::new(&path).exists() {
                parts.push(format!("[[ATTACH_IMAGE:{}]]", path));
            }
            return parts.join("\n\n");
        }

        // 1. Try to find a primary image from evidence to show inline (ChatGPT-style)
        if !self.evidence.is_empty() {
            for e in &self.evidence {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(e) {
                    if let Some(img_url) = value.get("image_url").and_then(|v| v.as_str()) {
                        if !img_url.is_empty() && !final_summary.contains(img_url) {
                            // STRIP common model refusals before adding the image
                            let lower = final_summary.to_lowercase();
                            if lower.contains("cannot directly show images")
                                || lower.contains("cannot display images")
                            {
                                final_summary = final_summary
                                    .replace("While I cannot directly show images, ", "");
                                final_summary = final_summary
                                    .replace("Since I cannot display images directly, ", "");
                                final_summary =
                                    final_summary.replace("I cannot show images directly. ", "");
                                final_summary = final_summary
                                    .replace("While I cannot display images directly, ", "");
                                final_summary = final_summary
                                    .replace("I cannot provide an image directly, ", "");
                            }

                            final_summary =
                                format!("{}\n\n![Source Image]({})", final_summary, img_url);
                            break; // Just one primary image is enough for the text bubble
                        }
                    }
                }
            }
        }

        let mut parts = vec![final_summary];

        fn format_json_if_possible(item: &str) -> String {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(item) {
                if let Some(obj) = value.as_object() {
                    let url = obj.get("url").and_then(|u| u.as_str()).unwrap_or("");
                    let title = obj
                        .get("title")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            if !url.is_empty() {
                                url.split('/')
                                    .nth(2)
                                    .unwrap_or("Source")
                                    .trim_start_matches("www.")
                                    .to_string()
                            } else {
                                "Source".to_string()
                            }
                        });
                    let snippet = obj.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
                    let image_url = obj.get("image_url").and_then(|i| i.as_str()).unwrap_or("");

                    let mut result = if !url.is_empty() {
                        if snippet.is_empty() {
                            format!("[{}]({})", title, url)
                        } else {
                            format!("[{}]({}): {}", title, url, snippet)
                        }
                    } else {
                        format!("{}: {}", title, snippet)
                    };

                    if !image_url.is_empty() {
                        result = format!("{}\n\n![{}]({})", result, title, image_url);
                    }
                    return result;
                }
            } else if item.contains("http://") || item.contains("https://") {
                // Heuristic: If it's not JSON but has a URL, ensure the URL is clickable
                let mut lines = item.lines();
                if let Some(first_line) = lines.next() {
                    let url = first_line.trim();
                    if url.starts_with("http") && url.len() < 512 {
                        let domain = url
                            .split('/')
                            .nth(2)
                            .unwrap_or("Source")
                            .trim_start_matches("www.");
                        return format!("[{}]({})", domain, url);
                    }
                }
            }
            item.to_string()
        }

        if !self.evidence.is_empty() {
            let mut link_lines: Vec<String> = Vec::new();

            for e in &self.evidence {
                if is_html_noise(e) {
                    continue;
                }
                // Try to parse as JSON object with url/title/snippet
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(e) {
                    if let Some(obj) = value.as_object() {
                        let url = obj.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        let title = obj
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("Source");
                        let snippet = obj.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
                        if !url.is_empty() {
                            if snippet.is_empty() {
                                link_lines.push(format!("- [{}]({})", title, url));
                            } else {
                                link_lines.push(format!("- [{}]({}): {}", title, url, snippet));
                            }
                        }
                        continue;
                    }
                    // Could be an array of results
                    if let Some(arr) = value.as_array() {
                        for item in arr {
                            if let Some(obj) = item.as_object() {
                                let url = obj.get("url").and_then(|u| u.as_str()).unwrap_or("");
                                let title = obj
                                    .get("title")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("Source");
                                let snippet =
                                    obj.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
                                if !url.is_empty() {
                                    if snippet.is_empty() {
                                        link_lines.push(format!("- [{}]({})", title, url));
                                    } else {
                                        link_lines
                                            .push(format!("- [{}]({}): {}", title, url, snippet));
                                    }
                                }
                            }
                        }
                        continue;
                    }
                }
                // Plain text with URL
                let text = e.trim();
                if text.starts_with("http") && text.len() < 512 {
                    let domain = text
                        .split('/')
                        .nth(2)
                        .unwrap_or("Source")
                        .trim_start_matches("www.");
                    link_lines.push(format!("- [{}]({})", domain, text));
                } else if !text.starts_with("---SEARCH_RESULTS---") && !text.is_empty() {
                    link_lines.push(format!("- {}", text));
                }
            }

            if !link_lines.is_empty() {
                parts.push(format!("**Sources**:\n{}", link_lines.join("\n")));
            }
        }

        if !self.artifacts.is_empty() {
            let formatted_artifacts: Vec<String> = self
                .artifacts
                .iter()
                .map(|a| {
                    let item = a.to_string();
                    let formatted = format_json_if_possible(&item);
                    // If it looks like a direct image URL, render it
                    let lower = item.to_lowercase();
                    if (item.starts_with("http://") || item.starts_with("https://"))
                        && (lower.ends_with(".png")
                            || lower.ends_with(".jpg")
                            || lower.ends_with(".jpeg")
                            || lower.ends_with(".webp")
                            || lower.ends_with(".svg"))
                    {
                        format!("![Artifact]({})", item)
                    } else {
                        formatted
                    }
                })
                .collect();
            parts.push(format!(
                "**Artifacts**:\n- {}",
                formatted_artifacts.join("\n- ")
            ));
        }

        if !self.verified && self.status == "completed" {
            parts.push(
                "> [!WARNING]\n\
                 > **Unverified**: The results above could not be fully grounded with current evidence."
                    .to_string(),
            );
        }

        if let Some(error) = &self.error {
            if !error.trim().is_empty() {
                parts.push(format!("Error: {}", error));
            }
        }

        parts.join("\n\n")
    }
}

fn is_native_screenshot_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") {
        return false;
    }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    (combined.contains("desktop") || combined.contains("screen"))
        && (combined.contains("screenshot")
            || combined.contains("screen capture")
            || combined.contains("capture screen")
            || combined.contains("capture my desktop")
            || combined.contains("take a screenshot"))
}

fn is_native_process_list_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") {
        return false;
    }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    let explicit_process_words = combined.contains("background task")
        || combined.contains("background tasks")
        || combined.contains("running process")
        || combined.contains("running processes")
        || combined.contains("process list")
        || combined.contains("active process")
        || combined.contains("active processes")
        || combined.contains("what's running")
        || combined.contains("whats running");
    let general_running_state = (combined.contains("currently running")
        || combined.contains("current running")
        || combined.contains("what is currently running")
        || combined.contains("what is running")
        || combined.contains("show what is running")
        || combined.contains("tell me what is running"))
        && (combined.contains("computer")
            || combined.contains("pc")
            || combined.contains("machine")
            || combined.contains("system"));

    (explicit_process_words || general_running_state)
        && !(combined.contains("how to") || combined.contains("manage background tasks"))
}

fn is_native_file_search_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") {
        return false;
    }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    (combined.contains("resume")
        || combined.contains("file path")
        || combined.contains("find file")
        || combined.contains("locate file")
        || combined.contains(".pdf")
        || combined.contains(".doc")
        || combined.contains(".docx")
        || ((combined.contains("find")
            || combined.contains("locate")
            || combined.contains("where is"))
            && (combined.contains("file") || combined.contains("path"))))
        && !combined.contains("open file explorer")
}

fn is_native_browser_state_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") {
        return false;
    }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    combined.contains("current tab")
        || combined.contains("current page")
        || combined.contains("browser state")
        || combined.contains("which browser")
        || (combined.contains("browser") && combined.contains("open"))
}

fn parse_browser_state_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let mut evidence = Vec::new();
        if let Some(windows) = value
            .get("browser_windows")
            .and_then(|item| item.as_array())
        {
            for window in windows.iter().take(12) {
                let name = window
                    .get("ProcessName")
                    .and_then(|item| item.as_str())
                    .unwrap_or("browser");
                let title = window
                    .get("MainWindowTitle")
                    .and_then(|item| item.as_str())
                    .unwrap_or("Untitled");
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

    (
        vec!["Browser windows were inspected natively.".to_string()],
        "Observed the current browser state.".to_string(),
    )
}

fn parse_file_search_payload(payload: &str) -> (Vec<String>, Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let mut evidence = Vec::new();
        let mut artifacts = Vec::new();
        if let Some(results) = value.get("results").and_then(|item| item.as_array()) {
            for result in results.iter().take(12) {
                let name = result
                    .get("name")
                    .and_then(|item| item.as_str())
                    .unwrap_or("unknown");
                let path = result
                    .get("path")
                    .and_then(|item| item.as_str())
                    .unwrap_or("");
                if !path.is_empty() {
                    evidence.push(format!("{name}: {path}"));
                    artifacts.push(path.to_string());
                }
            }
        }

        let summary = if let Some(first) = artifacts.first() {
            format!(
                "Found matching file paths on this PC. Best match: {}",
                first
            )
        } else {
            "No matching file path was found on this PC.".to_string()
        };

        return (evidence, artifacts, summary);
    }

    (
        vec!["Local file search completed.".to_string()],
        Vec::new(),
        "Local file search completed.".to_string(),
    )
}

fn parse_process_list_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let total = value
            .get("total_processes")
            .and_then(|item| item.as_i64())
            .unwrap_or(0);
        let background = value
            .get("background_count")
            .and_then(|item| item.as_i64())
            .unwrap_or(0);

        let mut evidence = Vec::new();
        if let Some(processes) = value
            .get("background_processes")
            .and_then(|item| item.as_array())
        {
            for process in processes.iter().take(12) {
                let name = process
                    .get("ProcessName")
                    .and_then(|item| item.as_str())
                    .unwrap_or("unknown");
                let pid = process
                    .get("Id")
                    .and_then(|item| item.as_i64())
                    .unwrap_or_default();
                let cpu = process
                    .get("CPU")
                    .and_then(|item| item.as_f64())
                    .unwrap_or(0.0);
                evidence.push(format!("{name} (PID {pid}, CPU {cpu:.1})"));
            }
        }

        let summary = if total > 0 {
            format!(
                "Observed {} running processes. Here are the current background tasks with the highest activity.",
                total
            )
        } else if background > 0 {
            format!(
                "Observed {} background processes. Here are the current background tasks with the highest activity.",
                background
            )
        } else {
            "Observed the current running background tasks.".to_string()
        };

        return (evidence, summary);
    }

    (
        vec!["Current running processes were inspected natively.".to_string()],
        "Observed the current running background tasks.".to_string(),
    )
}

fn parse_screenshot_artifact_payload(payload: &str) -> (Vec<String>, Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        let path = value
            .get("path")
            .and_then(|item| item.as_str())
            .unwrap_or("screenshot artifact");
        let width = value.get("width").and_then(|item| item.as_i64());
        let height = value.get("height").and_then(|item| item.as_i64());
        let summary = match (width, height) {
            (Some(w), Some(h)) => format!("Captured a desktop screenshot at {}x{}.", w, h),
            _ => "Captured a desktop screenshot.".to_string(),
        };
        let mut evidence = vec![format!("Native screenshot artifact created at {}.", path)];
        if let (Some(w), Some(h)) = (width, height) {
            evidence.push(format!("Screenshot dimensions: {}x{}.", w, h));
        }
        return (evidence, vec![path.to_string()], summary);
    }

    (
        vec!["Native screenshot artifact created.".to_string()],
        vec!["screenshot".to_string()],
        "Captured a desktop screenshot.".to_string(),
    )
}

pub async fn dispatch_domain(
    db: &crate::db::Database,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    agent_registry: &[AgentInfo],
    domain: &str,
    step: &str,
    query: &str,
    history: Vec<Message>,
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
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code, bypass_synthesizer, fast_path_keywords, verification_agent_id, routing_priority, hide_evidence, report_style, skip_fast_path_if_artifacts, artifact_fast_path_keywords FROM supervisors WHERE domain = ?"
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
                model: String::new(),
                domain: domain_name.to_string(),
                version: "1.0.0".to_string(),
                code: None,
                source_files: Vec::new(),
            })
    };

    let overseer_info = get_agent("overseer");
    let overseer_model = crate::backend::resolve_model_name(
        db,
        Some(&overseer_info.model),
        "default_chat_model",
        "qwen2.5:7b-instruct",
    )
    .await;
    let _overseer = OverseerAgent::new(ollama_client, &overseer_model, &overseer_info.instructions);

    println!("Dispatching dynamic supervisor for domain '{}'", domain);
    let s = supervisor
        .clone()
        .ok_or_else(|| format!("No supervisor found in DB for domain '{}'.", domain))?;
    println!(
        "Dispatching supervisor '{}' using model '{:?}'",
        s.name, s.model
    );
    let supervisor_model = crate::backend::resolve_model_name(
        db,
        s.model.as_deref(),
        "default_chat_model",
        "qwen2.5:7b-instruct",
    )
    .await;

    let tools = crate::backend::get_supervisor_tools(db, s.id)
        .await
        .unwrap_or_default();

    if is_native_screenshot_request(query, step, domain) {
        if let Some(os_tool) = tools.iter().find(|tool| tool.name == "os_tool") {
            let raw = crate::engine::tools::call_dynamic_tool(
                os_tool,
                serde_json::json!({ "action": "screenshot" }),
            )
            .await
            .map_err(|err| format!("Native screenshot capture failed: {}", err))?;

            let (evidence, artifacts, summary) = parse_screenshot_artifact_payload(&raw);

            record_event(
                db,
                run_id,
                event_sequence,
                "tool_started",
                "running",
                Some(domain),
                Some(step),
                Some("os_tool"),
                None,
                Some("Using native screenshot capture fast path."),
                Some(r#"{"action":"screenshot"}"#),
            )
            .await;

            record_event(
                db,
                run_id,
                event_sequence,
                "tool_completed",
                "completed",
                Some(domain),
                Some(step),
                Some("os_tool"),
                None,
                Some(&summary),
                Some(&raw),
            )
            .await;

            return Ok(SupervisorExecutionResult::completed(
                summary,
                evidence,
                artifacts,
                true,
                s.hide_evidence,
                raw,
            ));
        }
    }

    if is_native_process_list_request(query, step, domain) {
        if let Some(os_tool) = tools.iter().find(|tool| tool.name == "os_tool") {
            let raw = crate::engine::tools::call_dynamic_tool(
                os_tool,
                serde_json::json!({ "action": "list_processes" }),
            )
            .await
            .map_err(|err| format!("Native process inspection failed: {}", err))?;

            let (evidence, summary) = parse_process_list_payload(&raw);

            record_event(
                db,
                run_id,
                event_sequence,
                "tool_started",
                "running",
                Some(domain),
                Some(step),
                Some("os_tool"),
                None,
                Some("Using native running-process inspection fast path."),
                Some(r#"{"action":"list_processes"}"#),
            )
            .await;

            record_event(
                db,
                run_id,
                event_sequence,
                "tool_completed",
                "completed",
                Some(domain),
                Some(step),
                Some("os_tool"),
                None,
                Some(&summary),
                Some(&raw),
            )
            .await;

            return Ok(SupervisorExecutionResult::completed(
                summary,
                evidence,
                Vec::new(),
                true,
                s.hide_evidence,
                raw,
            ));
        }
    }

    if is_native_file_search_request(query, step, domain) {
        if let Some(os_tool) = tools.iter().find(|tool| tool.name == "os_tool") {
            let raw = crate::engine::tools::call_dynamic_tool(
                os_tool,
                serde_json::json!({ "action": "find_files", "query": query }),
            )
            .await
            .map_err(|err| format!("Native file search failed: {}", err))?;

            let (evidence, artifacts, summary) = parse_file_search_payload(&raw);

            record_event(
                db,
                run_id,
                event_sequence,
                "tool_started",
                "running",
                Some(domain),
                Some(step),
                Some("os_tool"),
                None,
                Some("Using native local-file search fast path."),
                Some(&format!(r#"{{"action":"find_files","query":{:?}}}"#, query)),
            )
            .await;

            record_event(
                db,
                run_id,
                event_sequence,
                "tool_completed",
                "completed",
                Some(domain),
                Some(step),
                Some("os_tool"),
                None,
                Some(&summary),
                Some(&raw),
            )
            .await;

            return Ok(SupervisorExecutionResult::completed(
                summary,
                evidence,
                artifacts,
                true,
                s.hide_evidence,
                raw,
            ));
        }
    }

    if is_native_browser_state_request(query, step, domain) {
        if let Some(os_tool) = tools.iter().find(|tool| tool.name == "os_tool") {
            let raw = crate::engine::tools::call_dynamic_tool(
                os_tool,
                serde_json::json!({ "action": "browser_state" }),
            )
            .await
            .map_err(|err| format!("Native browser inspection failed: {}", err))?;

            let (evidence, summary) = parse_browser_state_payload(&raw);

            record_event(
                db,
                run_id,
                event_sequence,
                "tool_started",
                "running",
                Some(domain),
                Some(step),
                Some("os_tool"),
                None,
                Some("Using native browser-state inspection fast path."),
                Some(r#"{"action":"browser_state"}"#),
            )
            .await;

            record_event(
                db,
                run_id,
                event_sequence,
                "tool_completed",
                "completed",
                Some(domain),
                Some(step),
                Some("os_tool"),
                None,
                Some(&summary),
                Some(&raw),
            )
            .await;

            return Ok(SupervisorExecutionResult::completed(
                summary,
                evidence,
                Vec::new(),
                true,
                s.hide_evidence,
                raw,
            ));
        }
    }

    let agent = AgentBuilder::new(ollama_client.completion_model(&supervisor_model))
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
             If you are returning a FINAL answer, respond ONLY with JSON:\n\
              {{\"status\":\"completed|unverified|failed\",\"summary\":\"concise overview\",\"evidence\":[...results...],\"artifacts\":[],\"verified\":true|false}}\n\
             EVIDENCE RULE: If stealth_search returned results, copy each result object as-is into the 'evidence' array as JSON objects with 'title', 'url', and 'snippet' fields.\n\
             SAFETY: Prefer lower-risk tools first.\n\
             When you call charting, its result (starting with ---CHART_SVG---) MUST be your COMPLETE final response.\n\
             Never claim completion without evidence.\n\n",
            list
        )
    };

    let has_charting_tool = tools.iter().any(|t| t.name == "charting");
    let clean_context = sanitize_context(memory_context);
    let clean_plan = sanitize_context(plan_history);
    let monitoring_hint = monitoring_strategy_hint(query, step, &tools)
        .map(|hint| format!("\n{}\n", hint))
        .unwrap_or_default();
    let computer_hint = computer_strategy_hint(query, step, &tools)
        .map(|hint| format!("\n{}\n", hint))
        .unwrap_or_default();

    // Fix 3: Elevate artifact context above the general CONTEXT block
    let (artifact_section, remaining_context) = if clean_context.contains("SELECTED ARTIFACTS:") {
        let artifact_header = crate::backend::get_setting_or(
            db,
            "artifact_context_header",
            "ATTACHED ARTIFACTS (the user explicitly selected these as context — use them to answer):",
        ).await;

        // Split the context into artifact block and remaining context
        if let Some(start_idx) = clean_context.find("SELECTED ARTIFACTS:") {
            // Find where the artifact block ends (next double newline or end of string)
            let artifact_block_start = start_idx;
            let after_header = start_idx + "SELECTED ARTIFACTS:".len();
            let artifact_block_end = clean_context[after_header..]
                .find("\n\n")
                .map(|pos| after_header + pos)
                .unwrap_or(clean_context.len());

            let artifact_content = clean_context[artifact_block_start + "SELECTED ARTIFACTS:".len()..artifact_block_end].trim();
            let before = clean_context[..artifact_block_start].trim();
            let after = clean_context[artifact_block_end..].trim();
            let remaining = format!("{}\n{}", before, after).trim().to_string();

            (
                format!("{}\n{}\n\n", artifact_header, artifact_content),
                remaining,
            )
        } else {
            (String::new(), clean_context)
        }
    } else {
        (String::new(), clean_context)
    };

    // Fix 4: Extract file attachment content from history and inject it into the prompt
    let mut attachment_content = String::new();
    for msg in &history {
        if let Message::User { content } = msg {
            for item in content.iter() {
                if let rig::message::UserContent::Text(text) = item {
                    // Parse [Attached File: filename (Path: path)] markers
                    for line in text.text.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("[Attached File:") || trimmed.starts_with("[Attached Image:") {
                            // Extract file path from the marker
                            if let Some(path_start) = trimmed.find("Path: ") {
                                let path_rest = &trimmed[path_start + 6..];
                                let path = path_rest.trim_end_matches(|c| c == ')' || c == ']');
                                // Try to read the file content
                                match std::fs::read_to_string(path) {
                                    Ok(content) if !content.trim().is_empty() => {
                                        let truncated = if content.len() > 8000 {
                                            format!("{}\n... [truncated, {} total chars]", &content[..8000], content.len())
                                        } else {
                                            content
                                        };
                                        attachment_content.push_str(&format!(
                                            "\n--- FILE: {} ---\n{}\n--- END FILE ---\n",
                                            path, truncated
                                        ));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let file_section = if !attachment_content.is_empty() {
        let header = crate::backend::get_setting_or(
            db,
            "artifact_context_header",
            "ATTACHED ARTIFACTS (the user explicitly selected these as context — use them to answer):",
        ).await;
        format!("{}\n{}\n\n", header, attachment_content)
    } else {
        String::new()
    };

    let mut initial_prompt = format!(
        "GOAL: {}\n\
         TASK: {}\n\n\
         {}{}{}CONTEXT:\n{}\n\n\
         PROGRESS:\n{}\n\n\
         COMMAND: Execute the task using tools if necessary. Final answers must use the structured JSON result schema.{}{}",
        query, step, artifact_section, file_section, tool_descriptions, remaining_context, clean_plan, monitoring_hint, computer_hint
    );

    if domain == "chat" {
        // No hard-coded logic needed, instructions are in DB
    }

    if domain == "stealth" {
        // No hard-coded logic needed, instructions are in DB
    }

    let s = match sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code, bypass_synthesizer, fast_path_keywords, verification_agent_id, routing_priority, hide_evidence, report_style, skip_fast_path_if_artifacts, artifact_fast_path_keywords FROM supervisors WHERE domain = ?"
    )
    .bind(domain)
    .fetch_optional(&db.pool)
    .await {
        Ok(Some(s)) => s,
        _ => {
            return Ok(SupervisorExecutionResult::failed(
                "Execution failed.".to_string(),
                format!("Domain supervisor '{}' not found in database.", domain),
                String::new(),
            ));
        }
    };

    record_event(
        db,
        run_id,
        event_sequence,
        "supervisor_started",
        "running",
        Some(domain),
        Some(step),
        None,
        s.model.as_deref(),
        Some(&format!(
            "Supervisor '{}' starting for domain '{}'.",
            s.name, domain
        )),
        None,
    )
    .await;

    println!("Prompt text:\n{}", initial_prompt);
    println!(
        "Supervisor preamble:\n{}",
        s.instructions.as_deref().unwrap_or("")
    );

    let base_iterations = crate::backend::get_setting_i64(db, "max_iterations", 10)
        .await
        .max(1) as usize;
    let hard_iteration_cap = crate::backend::get_setting_i64(
        db,
        "max_iterations_hard_cap",
        (base_iterations as i64).max(10) * 3,
    )
    .await
    .max(base_iterations as i64) as usize;
    let tool_timeout_ms = crate::backend::get_setting_i64(db, "tool_timeout_ms", 45_000)
        .await
        .max(1) as u64;

    let mut current_prompt = initial_prompt;
    let mut final_response: Option<SupervisorExecutionResult> = None;
    let mut manual_svg_retries = 0;
    let mut last_tool_signature: Option<String> = None;
    let mut repeated_tool_calls = 0usize;
    let mut tool_progress_signatures = HashSet::new();
    let mut iteration_budget = base_iterations;
    const MAX_SVG_RETRIES: usize = 2;
    const MAX_IDENTICAL_TOOL_CALLS: usize = 3;
    const ITERATION_EXTENSION_CHUNK: usize = 3;

    let mut iteration = 0usize;
    while iteration < iteration_budget && iteration < hard_iteration_cap {
        println!(
            "Supervisor iteration {}/{} (hard cap {})",
            iteration + 1,
            iteration_budget,
            hard_iteration_cap
        );

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
            Some(&supervisor_model),
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
                let tool_signature = format!(
                    "{}::{}",
                    tool_name,
                    serde_json::to_string(&input_json).unwrap_or_else(|_| input_json.to_string())
                );

                if last_tool_signature.as_ref() == Some(&tool_signature) {
                    repeated_tool_calls += 1;
                } else {
                    last_tool_signature = Some(tool_signature.clone());
                    repeated_tool_calls = 1;
                }

                let new_progress_signature =
                    tool_progress_signatures.insert(tool_signature.clone());
                if new_progress_signature
                    && iteration_budget < hard_iteration_cap
                    && iteration + 2 >= iteration_budget
                {
                    let previous_budget = iteration_budget;
                    iteration_budget =
                        (iteration_budget + ITERATION_EXTENSION_CHUNK).min(hard_iteration_cap);

                    if iteration_budget > previous_budget {
                        let details = format!(
                            "Progress detected through a new tool path. Extending supervisor budget from {} to {} iterations.",
                            previous_budget, iteration_budget
                        );
                        log_event(
                            db,
                            run_id,
                            event_sequence,
                            "supervisor_budget_extended",
                            "running",
                            Some(domain),
                            Some(step),
                            Some(tool_name),
                            s.model.as_deref(),
                            Some("Supervisor earned more runtime after making progress."),
                            Some(&details),
                        )
                        .await;
                    }
                }

                if repeated_tool_calls >= 2 {
                    let alternate_tool = suggest_alternate_tool(tool_name, &tools);
                    let monitoring_override = if is_monitoring_task(query, step) {
                        if let Some(url) =
                            extract_first_url(step).or_else(|| extract_first_url(query))
                        {
                            Some(format!(
                                "This is a monitoring task. Stop guessing alternate sites and verify the exact target URL '{}'. Prefer http_request with {{\"url\":\"{}\",\"method\":\"GET\"}} if available.",
                                url, url
                            ))
                        } else {
                            Some(
                                "This is a monitoring task and no exact target URL is present. Do not guess unrelated sites. Either search once for the official status page or return a failed JSON result asking for the concrete website URL."
                                    .to_string(),
                            )
                        }
                    } else {
                        None
                    };

                    let override_message = if let Some(message) = monitoring_override {
                        message
                    } else if let Some(alternate_tool) = alternate_tool {
                        format!(
                            "SYSTEM OVERRIDE: You have repeated the same tool call '{}' without making progress. Do NOT call '{}' again with the same input. Switch strategy now by using '{}' if it can advance the task, or return a final JSON result that explains the failure with evidence.",
                            tool_name, tool_name, alternate_tool
                        )
                    } else {
                        format!(
                            "SYSTEM OVERRIDE: You have repeated the same tool call '{}' without making progress. Do NOT call it again with the same input. Either choose a different tool or return a final JSON result that explains the failure with evidence.",
                            tool_name
                        )
                    };

                    log_event(
                        db,
                        run_id,
                        event_sequence,
                        "tool_loop_warning",
                        if repeated_tool_calls >= MAX_IDENTICAL_TOOL_CALLS {
                            "error"
                        } else {
                            "warning"
                        },
                        Some(domain),
                        Some(step),
                        Some(tool_name),
                        s.model.as_deref(),
                        Some("Supervisor repeated the same tool call and was asked to change strategy."),
                        Some(&tool_signature),
                    )
                    .await;

                    if repeated_tool_calls < MAX_IDENTICAL_TOOL_CALLS {
                        current_prompt = format!(
                            "{}\n\n[{}]\n\nNext: either call a different tool with JSON or return your final structured JSON result.",
                            current_prompt, override_message
                        );
                        iteration += 1;
                        continue;
                    }
                }

                if repeated_tool_calls >= MAX_IDENTICAL_TOOL_CALLS {
                    let loop_error = format!(
                        "Supervisor repeated the same tool call '{}' {} times without making progress.",
                        tool_name, repeated_tool_calls
                    );

                    log_event(
                        db,
                        run_id,
                        event_sequence,
                        "tool_loop_detected",
                        "error",
                        Some(domain),
                        Some(step),
                        Some(tool_name),
                        s.model.as_deref(),
                        Some("Supervisor is stuck repeating the same tool call."),
                        Some(&tool_signature),
                    )
                    .await;

                    final_response = Some(SupervisorExecutionResult::failed(
                        "Supervisor entered a repeated tool-call loop.".to_string(),
                        loop_error,
                        trimmed.to_string(),
                    ));
                    break;
                }

                let tool_info = tools.iter().find(|t| t.name == tool_name);
                let computer_gate = match tool_info {
                    Some(info) => Some(
                        evaluate_computer_tool_gate(
                            db,
                            domain,
                            tool_name,
                            &info.safety_level,
                            &input_json,
                        )
                        .await,
                    ),
                    None => None,
                };

                if let Some(info) = tool_info {
                    if let Some(gate) = computer_gate.as_ref() {
                        if !gate.allowed {
                            let reason = gate.reason.clone().unwrap_or_else(|| {
                                "Computer supervision blocked this action.".to_string()
                            });
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
                                "{}\n\n[SYSTEM OVERRIDE: {}]\n\nNext: Ask for approval or provide your answer using the structured JSON result schema.",
                                current_prompt, reason
                            );
                            iteration += 1;
                            continue;
                        }
                    }
                    let guardrail =
                        super::guardrails::check_tool_guardrails(query, step, info, &input_json);

                    if !guardrail.allowed {
                        let reason = guardrail.reason.unwrap_or_else(|| {
                            "Tool execution blocked by safety guardrails.".to_string()
                        });
                        log_event(
                            db,
                            run_id,
                            event_sequence,
                            "guardrail_intercept",
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
                            "{}\n\n[GUARDRAIL INTERCEPT: {}]\n\nNext: Explain to the user why this specific tool/intent is needed, or choose an alternate strategy.",
                            current_prompt, reason
                        );
                        iteration += 1;
                        continue;
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
                    Some(info) => match tokio::time::timeout(
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
                    },
                    None => format!("Tool '{}' not found.", tool_name),
                };

                if let Some(gate) = computer_gate.as_ref() {
                    let tool_succeeded = !tool_result.starts_with("Tool error:");
                    if gate.allowed && gate.action_tool && tool_succeeded {
                        let action_summary = gate
                            .action_summary
                            .clone()
                            .unwrap_or_else(|| tool_name.to_string());
                        let _ = crate::backend::register_computer_action_execution(
                            db,
                            tool_name,
                            &action_summary,
                        )
                        .await;
                    }
                }

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
                        s.hide_evidence,
                        tool_result,
                    ));
                    break;
                }

                current_prompt = format!(
                    "{}\n\n[RESULT for '{}' ({}):\n{}]\n\nNext: Call another tool (JSON) or provide your final answer using the structured JSON result schema.",
                    current_prompt, tool_name, input_val, tool_result
                );
                iteration += 1;
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
                iteration += 1;
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
        final_response = Some(parse_supervisor_result(processed, s.hide_evidence));
        break;
    }

    Ok(final_response.unwrap_or_else(|| {
        SupervisorExecutionResult::failed(
            "Supervisor stopped without a final result.".to_string(),
            format!(
                "Supervisor exhausted its persistence budget after {} iterations without reaching a final response. Hard cap: {}.",
                iteration,
                hard_iteration_cap
            ),
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

fn suggest_alternate_tool<'a>(
    current_tool: &str,
    tools: &'a [crate::engine::utils::discovery::ToolInfo],
) -> Option<&'a str> {
    let has = |name: &str| tools.iter().any(|tool| tool.name == name);

    match current_tool {
        "stealth_browse" | "stealth_search" if has("http_request") => Some("http_request"),
        "http_request" if has("stealth_browse") => Some("stealth_browse"),
        _ => None,
    }
}

fn extract_first_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|token| {
            token.trim_matches(|c: char| matches!(c, '"' | '\'' | '(' | ')' | '[' | ']' | ','))
        })
        .find(|token| token.starts_with("http://") || token.starts_with("https://"))
        .map(|token| token.to_string())
}

fn is_monitoring_task(query: &str, step: &str) -> bool {
    let combined = format!("{} {}", query, step).to_lowercase();
    [
        "status",
        "uptime",
        "monitor",
        "monitoring",
        "healthcheck",
        "up/down",
        "availability",
    ]
    .iter()
    .any(|keyword| combined.contains(keyword))
}

fn monitoring_strategy_hint(
    query: &str,
    step: &str,
    tools: &[crate::engine::utils::discovery::ToolInfo],
) -> Option<String> {
    if !is_monitoring_task(query, step) {
        return None;
    }

    let has_http_request = tools.iter().any(|tool| tool.name == "http_request");
    let has_stealth_search = tools.iter().any(|tool| tool.name == "stealth_search");
    let explicit_url = extract_first_url(step).or_else(|| extract_first_url(query));

    if let Some(url) = explicit_url {
        if has_http_request {
            return Some(format!(
                "MONITORING OVERRIDE: This is a website monitoring task and a concrete URL is already available. Prefer a direct HTTP check first with JSON like {{\"tool\":\"http_request\",\"input\":{{\"url\":\"{}\",\"method\":\"GET\"}}}}. Only use stealth browsing if the page is reachable and you still need cleaned text for evidence.",
                url
            ));
        }

        return Some(format!(
            "MONITORING OVERRIDE: Use the provided URL '{}' as the primary target. Do not guess unrelated third-party sites. If you cannot verify the status of that exact URL, return a failed or unverified JSON result with the reason.",
            url
        ));
    }

    if has_stealth_search {
        return Some(
            "MONITORING OVERRIDE: No concrete URL is present. Do not guess random third-party status sites. Use stealth_search once to find the official site or status page for the target service. If the target URL remains unknown, return a failed JSON result asking for a concrete website or status-page URL."
                .to_string(),
        );
    }

    Some(
        "MONITORING OVERRIDE: A concrete target URL is missing. Do not invent one. Return a failed JSON result asking for the exact website or status-page URL."
            .to_string(),
    )
}

pub fn parse_supervisor_result(cleaned: String, hide_evidence: bool) -> SupervisorExecutionResult {
    if cleaned.trim().is_empty() {
        return SupervisorExecutionResult::failed(
            "Empty response from supervisor.".to_string(),
            "Empty response.".to_string(),
            cleaned,
        );
    }

    if cleaned.contains("---chart_svg---") {
        return SupervisorExecutionResult::completed(
            "Generated a chart artifact.".to_string(),
            vec!["Charting tool returned a chart artifact.".to_string()],
            vec!["chart".to_string()],
            true,
            hide_evidence,
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
            let mut summary = value
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or(&cleaned)
                .to_string();

            if summary.trim().is_empty() {
                summary = "Research results found. See sources for details.".to_string();
            }

            // If summary itself is a JSON object string, it's a leakage
            if summary.starts_with('{') && summary.contains("\"status\"") {
                summary = "Research completed. See sources below.".to_string();
            }

            let evidence = value
                .get("evidence")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            if item.is_string() {
                                item.as_str().map(|s| s.to_string())
                            } else {
                                Some(item.to_string())
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let artifacts = value
                .get("artifacts")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            if item.is_string() {
                                item.as_str().map(|s| s.to_string())
                            } else {
                                Some(item.to_string())
                            }
                        })
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
                hide_evidence,
                error,
                raw_response: cleaned,
            };
        }
    }

    // BEST EFFORT PARSING for malformed or truncated JSON
    println!("⚠️ FAILED to parse supervisor JSON. Attempting best-effort extraction...");
    let status = if cleaned.contains("\"status\":\"failed\"") {
        "failed"
    } else {
        "completed"
    };

    // Extract summary using a simple string search
    let mut summary = "Research completed. See sources below for details.".to_string();
    if let Some(start) = cleaned.find("\"summary\":\"") {
        let after = &cleaned[start + 11..];
        if let Some(end) = after.find("\",\"") {
            summary = after[..end].to_string();
        } else if let Some(end) = after.find("\"") {
            summary = after[..end].to_string();
        }
    }

    // Extract evidence URLs if possible
    let mut evidence = Vec::new();
    let mut current = &cleaned[..];
    while let Some(start) = current.find("\"url\":\"") {
        let after = &current[start + 7..];
        if let Some(end) = after.find("\"") {
            evidence.push(after[..end].to_string());
            current = &after[end..];
        } else {
            break;
        }
    }

    SupervisorExecutionResult {
        status: status.to_string(),
        summary,
        evidence,
        artifacts: Vec::new(),
        verified: false,
        hide_evidence,
        error: None,
        raw_response: cleaned,
    }
}

fn post_process_for_domain(_domain: &str, response: String) -> String {
    post_process_agent_response(response)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_request_detectors_choose_computer_fast_paths() {
        assert!(is_native_screenshot_request(
            "Take a screenshot of my desktop",
            "Capture the desktop",
            "computer"
        ));
        assert!(is_native_process_list_request(
            "Tell me the background tasks that I'm currently running",
            "Inspect running processes",
            "computer"
        ));
        assert!(is_native_process_list_request(
            "What is currently running on my computer",
            "Inspect current machine activity",
            "computer"
        ));
        assert!(is_native_file_search_request(
            "Find my resume path on this PC",
            "Search local files",
            "computer"
        ));
        assert!(is_native_browser_state_request(
            "Which browser windows are open right now?",
            "Inspect browser state",
            "computer"
        ));
    }

    #[test]
    fn screenshot_payload_becomes_grounded_chat_summary() {
        let payload = r#"{"action":"screenshot","path":"C:\\temp\\desktop-test.png","width":1536,"height":864}"#;
        let (evidence, artifacts, summary) = parse_screenshot_artifact_payload(payload);
        assert_eq!(artifacts, vec!["C:\\temp\\desktop-test.png".to_string()]);
        assert!(summary.contains("1536x864"));
        assert!(evidence
            .iter()
            .any(|item| item.contains("Native screenshot artifact created")));
    }

    #[test]
    fn screenshot_display_text_stays_clean() {
        let screenshot_path = std::env::temp_dir().join("overseer-test-screenshot.png");
        std::fs::write(&screenshot_path, b"test").expect("temp screenshot file");
        let result = SupervisorExecutionResult::completed(
            "### Screenshot of Your Desktop".to_string(),
            vec![
                "Screenshot dimensions: 1536x864.".to_string(),
                format!("<img src='{}'>", screenshot_path.display()),
            ],
            vec![screenshot_path.to_string_lossy().to_string()],
            true,
            false,
            format!(
                r#"{{"action":"screenshot","path":"{}","width":1536,"height":864}}"#,
                screenshot_path.to_string_lossy()
            ),
        );

        let display = result.display_text();
        assert!(display.contains("Screenshot of Your Desktop"));
        assert!(display.contains("Screenshot dimensions: 1536x864."));
        assert!(display.contains("Screenshot attached below."));
        assert!(display.contains("[[ATTACH_IMAGE:"));
        assert!(!display.contains("###"));
        assert!(!display.contains("<img"));
        let _ = std::fs::remove_file(screenshot_path);
    }

    #[test]
    fn process_payload_prefers_observed_process_summary() {
        let payload = r#"{
            "action":"list_processes",
            "total_processes":42,
            "background_count":12,
            "background_processes":[
                {"ProcessName":"ollama","Id":11434,"CPU":3.2,"WS":1234},
                {"ProcessName":"code","Id":999,"CPU":1.2,"WS":5678}
            ]
        }"#;

        let (evidence, summary) = parse_process_list_payload(payload);
        assert!(summary.contains("42 running processes"));
        assert!(evidence.iter().any(|item| item.contains("ollama")));
        assert!(evidence.iter().any(|item| item.contains("PID 11434")));
    }
}
