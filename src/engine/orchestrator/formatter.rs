use crate::engine::orchestrator::result::SupervisorExecutionResult;
use super::response::{StructuredResponse, MessagePart, SourcePart};
use super::format_helpers::{normalize_summary, extract_artifact_path, is_html_noise};

pub fn format_supervisor_result(res: &SupervisorExecutionResult) -> StructuredResponse {
    let mut structured = StructuredResponse::new();
    let final_summary = normalize_summary(&res.summary);

    // Main message
    if !final_summary.is_empty() {
        structured.messages.push(MessagePart {
            message: final_summary,
            role: "assistant".to_string(),
        });
    }

    // Sources (evidence)
    for e in &res.evidence {
        if is_html_noise(e) { continue; }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(e) {
            if let Some(obj) = value.as_object() {
                let url = obj.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let title = obj.get("title").and_then(|t| t.as_str()).unwrap_or("Source");
                let snippet = obj.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
                if !url.is_empty() {
                    structured.sources.push(SourcePart {
                        source: title.to_string(),
                        link: url.to_string(),
                        description: snippet.to_string(),
                    });
                }
            }
        }
    }

    // Artifacts & Attachments
    for a in &res.artifacts {
        if let Some(path) = extract_artifact_path(a) {
            if path.contains("assets/") || path.contains("/") || path.contains("\\") {
                structured.attachments.push(path);
            } else {
                structured.artifacts.push(path);
            }
        } else {
            structured.artifacts.push(a.to_string());
        }
    }

    // Screenshots from evidence (legacy support)
    for e in &res.evidence {
        if let Some(path) = extract_artifact_path(e) {
            if !structured.attachments.contains(&path) {
                structured.attachments.push(path);
            }
        }
    }

    // 5. Append charts from intermediate result
    structured.charts.extend(res.charts.clone());

    structured
}
