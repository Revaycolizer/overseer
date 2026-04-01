use crate::db::Database;
use serde_json::Value;
use super::native::{ToolGateDecision};

pub fn is_computer_observation_tool(tool_name: &str) -> bool {
    matches!(tool_name, "vision_analysis")
}

pub fn is_computer_action_tool(domain: &str, tool_name: &str, safety_level: &str) -> bool {
    if tool_name == "os_tool" { return true; }
    if matches!(safety_level, "high" | "dangerous") && matches!(domain, "computer" | "desktop") {
        return true;
    }
    matches!(domain, "computer" | "desktop") && !is_computer_observation_tool(tool_name)
}

pub fn summarize_computer_action(tool_name: &str, input_json: &Value) -> String {
    if let Some(input) = input_json.get("input").and_then(|value| value.as_str()) {
        return format!("{} -> {}", tool_name, input.trim());
    }
    if let Some(action) = input_json.get("action").and_then(|value| value.as_str()) {
        if matches!(action, "inspect_accessibility" | "focus_element" | "invoke_element") {
            let selector = input_json.get("selector").and_then(|v| v.as_str()).map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .or_else(|| {
                    let mut parts = Vec::new();
                    for (label, key) in [("name", "name"), ("automation_id", "automation_id"), ("class", "class_name"), ("control_type", "control_type")] {
                        if let Some(value) = input_json.get(key).and_then(|v| v.as_str()) {
                            let trimmed = value.trim();
                            if !trimmed.is_empty() { parts.push(format!("{}={}", label, trimmed)); }
                        }
                    }
                    if parts.is_empty() { None } else { Some(parts.join(", ")) }
                });
            if let Some(selector) = selector { return format!("{} -> {} ({})", tool_name, action.trim(), selector); }
        }
    }
    if let Some(action) = input_json.get("action").and_then(|value| value.as_str()) {
        if let Some(app) = input_json.get("app").and_then(|value| value.as_str()) {
            return format!("{} -> {} ({})", tool_name, action.trim(), app.trim());
        }
        return format!("{} -> {}", tool_name, action.trim());
    }
    format!("{} -> {}", tool_name, input_json)
}

pub fn computer_strategy_hint(query: &str, step: &str, tools: &[crate::engine::utils::discovery::ToolInfo]) -> Option<String> {
    let lower = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    if !(lower.contains("computer") || lower.contains("desktop") || lower.contains("window") || lower.contains("screen")) { return None; }
    let has_os_tool = tools.iter().any(|tool| tool.name == "os_tool");
    let has_vision_tool = tools.iter().any(|tool| tool.name == "vision_analysis");
    if !has_os_tool && !has_vision_tool { return None; }

    let mut guidance = Vec::new();
    if lower.contains("screenshot") || lower.contains("screen capture") || lower.contains("capture screen") {
        if has_os_tool {
            guidance.push("For screenshot tasks, call `os_tool` with JSON like {\"action\":\"screenshot\"}. Do not open Snipping Tool or any other UI.");
        }
        guidance.push("A native screenshot artifact counts as completion for a screenshot request.");
    }
    if guidance.is_empty() { None } else { Some(guidance.join(" ")) }
}

pub async fn evaluate_computer_tool_gate(db: &Database, domain: &str, tool_name: &str, safety_level: &str, input_json: &Value) -> ToolGateDecision {
    let action_tool = is_computer_action_tool(domain, tool_name, safety_level);
    let touches_computer = matches!(domain, "computer" | "desktop") || action_tool || is_computer_observation_tool(tool_name);
    if !touches_computer { return ToolGateDecision { allowed: true, reason: None, action_summary: None, action_tool: false }; }

    let state = crate::backend::get_computer_supervision_state(db).await;
    let action_summary = summarize_computer_action(tool_name, input_json);

    if state.session_state == "emergency_stopped" {
        return ToolGateDecision { allowed: false, reason: Some("Computer supervision is in emergency stop.".to_string()), action_summary: Some(action_summary), action_tool };
    }
    if state.session_state == "paused" {
        return ToolGateDecision { allowed: false, reason: Some("Computer supervision is paused.".to_string()), action_summary: Some(action_summary), action_tool };
    }
    if action_tool && state.session_state == "idle" {
        return ToolGateDecision { allowed: false, reason: Some("No supervised computer session is active.".to_string()), action_summary: Some(action_summary), action_tool };
    }
    
    ToolGateDecision { allowed: true, reason: None, action_summary: Some(action_summary), action_tool }
}
