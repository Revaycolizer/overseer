use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::response::{StructuredResponse, MessagePart, SourcePart};
use crate::engine::orchestrator::utils::format_helpers::{normalize_summary, extract_artifact_path, is_html_noise};

pub fn format_supervisor_result(res: &SupervisorExecutionResult) -> StructuredResponse {
    let mut structured = StructuredResponse::new();
    let final_summary = normalize_summary(&res.summary);
    
    structured.status = res.status.clone();

    // Main message
    // Main message
    let mut message = final_summary.clone();
    let is_generic = message == "Task completed." || message == "Done." || message == "Finished." || message.is_empty();

    // STYLE-AWARE PROMOTION: If message is generic but we have findings, promote them.
    if is_generic && !res.evidence.is_empty() {
        let findings_label = if res.report_style.as_deref() == Some("concise") {
            "Found findings: "
        } else {
            "\n\n### Findings:\n"
        };
        
        let mut findings_str = String::from(findings_label);
        if res.report_style.as_deref() == Some("concise") {
             findings_str.push_str(&res.evidence.join(", "));
        } else {
             for e in &res.evidence {
                 findings_str.push_str(&format!("- {}\n", e));
             }
        }
        
        if message == "Task completed." || message == "Done." || message == "Finished." {
            message = format!("{}{}", message, findings_str);
        } else {
            message = findings_str;
        }
    }

    if !message.is_empty() {
        let is_generic_now = message == "Task completed." || message == "Done." || message == "Finished.";
        // Skip generic summary if we have better content coming (charts, etc.)
        if !is_generic_now || (res.charts.is_empty() && res.artifacts.is_empty()) {
            structured.messages.push(MessagePart {
                message,
                role: "agent".to_string(),
            });
        }
    }

    // 2. Evidence -> Sources
    for e in &res.evidence {
        if is_html_noise(e) {
            continue;
        }
        
        // Try to parse as JSON if it's already structured evidence
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(e) {
            let desc = v.get("description").or(v.get("summary")).and_then(|v| v.as_str()).unwrap_or(e);
            let link = v.get("link").or(v.get("url")).and_then(|v| v.as_str()).unwrap_or("");
            let source = v.get("source").or(v.get("title")).and_then(|v| v.as_str()).unwrap_or("Tool Output");
            
            structured.sources.push(SourcePart {
                source: source.to_string(),
                link: link.to_string(),
                description: desc.to_string(),
            });
            continue;
        }

        // Fallback: If it's a plain string, wrap it as a description
        if !e.trim().is_empty() {
            structured.sources.push(SourcePart {
                source: "Evidence".to_string(),
                link: "".to_string(),
                description: e.clone(),
            });
        }
    }

    // 3. Artifacts
    for a in &res.artifacts {
        if let Some(path) = extract_artifact_path(a) {
            structured.artifacts.push(path);
        }
    }

    // 4. Attachments (Legacy/Additional)
    for a in &res.artifacts {
        if a.contains("assets/") || a.contains("./assets") {
             structured.attachments.push(a.clone());
        }
    }

    // 5. Append charts from intermediate result (Data-driven)
    structured.charts.extend(res.charts.clone());

    structured
}
