use serde_json::Value;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::response::{ChartPart, MessagePart};

pub fn extract_result_from_value(value: Value, summary: String, raw: String, hide_evidence: bool, report_style: Option<String>) -> SupervisorExecutionResult {
    let target = value.get("final").unwrap_or(&value);
    
    let mut evidence = Vec::new();
    if let Some(e) = target.get("evidence").or(target.get("evidences")).and_then(|v| v.as_array()) {
        evidence = e.iter().filter_map(|v| {
            if let Some(s) = v.as_str() { Some(s.to_string()) }
            else if v.is_object() { Some(v.to_string()) }
            else { None }
        }).collect();
    }
    
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
        let is_verified = if let Some(b) = v.as_bool() { b } else {
            let lower = s_val.to_lowercase();
            lower == "completed" || lower == "done" || lower == "success" || lower == "verified"
        };
        (is_verified, s_val)
    } else { (true, "completed".to_string()) };

    SupervisorExecutionResult::new(status, final_summary.trim().to_string(), messages, evidence, artifacts, charts, verified, hide_evidence, report_style, raw, findings)
}
