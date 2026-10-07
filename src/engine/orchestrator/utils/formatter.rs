use crate::engine::orchestrator::core::response::{MessagePart, SourcePart, StructuredResponse};
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::utils::format_helpers::{
    extract_artifact_path, is_html_noise, normalize_summary,
};
use crate::engine::orchestrator::utils::json_strip::strip_embedded_json;

pub fn format_supervisor_result(res: &SupervisorExecutionResult) -> StructuredResponse {
    let mut structured = StructuredResponse::new();
    let final_summary = normalize_summary(&res.summary);

    structured.status = res.status.clone();

    // 1. Messages Strategy:
    // If specifically provided by the supervisor, use them.
    if !res.messages.is_empty() {
        // Some small models (qwen2.5 7b in particular) wrap their
        // structured response as plain text inside `message` instead of
        // emitting it as the proper JSON shape. Strip any embedded
        // JSON shell so the renderer shows just the prose answer
        // instead of "answer + duplicate JSON".
        structured
            .messages
            .extend(res.messages.iter().cloned().map(|mut m| {
                m.message = strip_embedded_json(&m.message);
                m
            }));
    } else {
        // Fallback synthesis from summary and evidence
        let mut message = final_summary.clone();
        let lower = message.to_lowercase();
        let is_generic = lower == "task completed."
            || lower == "done."
            || lower == "finished."
            || lower == "user task completed"
            || lower == "task completed"
            || message.is_empty();

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

            if lower == "task completed."
                || lower == "done."
                || lower == "finished."
                || lower == "user task completed"
                || lower == "task completed"
            {
                message = format!("{}{}", message, findings_str);
            } else {
                message = findings_str;
            }
        }

        // ARTIFACT PROMOTION: Ensure artifact links are always in the message
        if !res.artifacts.is_empty() {
            let art_msg = format!("Generated document: {}", res.artifacts.join(", "));
            if is_generic || message.is_empty() {
                message = art_msg;
            } else if !message.contains("Generated document") {
                message = format!("{}\n\n{}", message, art_msg);
            }
        }

        if !message.is_empty() {
            let is_generic_now = message.to_lowercase().contains("task completed")
                || message.to_lowercase().contains("done.")
                || message.is_empty();
            // Skip generic summary if we have better content coming (charts, etc.)
            if !is_generic_now || (res.charts.is_empty() && res.artifacts.is_empty()) {
                structured.messages.push(MessagePart {
                    message,
                    role: "assistant".to_string(),
                });
            }
        }
    }

    // 2. Evidence -> Sources
    if !res.hide_evidence {
        for e in &res.evidence {
            if is_html_noise(e) {
                continue;
            }

            // Try to parse as JSON if it's already structured evidence (e.g. from a search tool)
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(e) {
                let title = v
                    .get("title")
                    .or(v.get("source"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Evidence");
                let link = v
                    .get("url")
                    .or(v.get("link"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let desc = v
                    .get("snippet")
                    .or(v.get("description"))
                    .or(v.get("summary"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(e);

                structured.sources.push(SourcePart {
                    source: title.to_string(),
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

/// V4.17.10: render a `StructuredResponse` as a self-contained
/// Markdown document for the conversation exporter. The
/// existing `format_supervisor_result` focuses on UI widgets (blocks,
/// sources) — this variant is a pure-Markdown rendering so
/// the same `StructuredResponse` data can be archived or shared as
/// `.md`.
pub fn format_for_markdown(res: &StructuredResponse) -> String {
    let mut out = String::new();
    for m in &res.messages {
        out.push_str(&format!(
            "**{}:** {}\n\n",
            m.role,
            m.message.replace("```", "ʼʼʼ")
        ));
    }
    if !res.sources.is_empty() {
        out.push_str("### Sources\n\n");
        for s in &res.sources {
            out.push_str(&format!("- **{}**", s.source));
            if !s.link.is_empty() {
                out.push_str(&format!(" ([{}])", s.link));
            }
            if !s.description.is_empty() {
                out.push_str(&format!(" — {}", s.description));
            }
            out.push_str("\n");
        }
        out.push('\n');
    }
    for (i, c) in res.charts.iter().enumerate() {
        out.push_str(&format!(
            "### Chart {}: {}\n\n\
             ```\n\
             type: {}\n\
             labels: {:?}\n\
             data:   {:?}\n\
             ```\n\n",
            i + 1,
            c.title.as_deref().unwrap_or(""),
            c.chart_type,
            c.labels.as_deref().unwrap_or(&Vec::new()),
            c.data.as_deref().unwrap_or(&Vec::new()),
        ));
    }
    out
}
