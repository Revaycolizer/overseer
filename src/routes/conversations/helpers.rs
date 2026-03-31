use crate::backend;
use dioxus::prelude::document::eval;
use serde::Serialize;
pub struct ParsedMessage {
    pub display_text: String,
    pub attachments: Vec<String>, // or a richer type
}
#[derive(Debug, Clone, Serialize)]
struct ArtifactReferencePayload {
    id: i64,
    label: String,
    kind: String,
    domain: Option<String>,
    file_path: Option<String>,
    content_type: Option<String>,
    content_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct WorkItemReferencePayload {
    id: i64,
    title: String,
    brief: Option<String>,
    status: String,
    surface_kind: String,
    assigned_domain: Option<String>,
    prompt: String,
}

pub fn build_query_with_context(
    query: &str,
    artifacts: &[backend::Artifact],
    work_item: Option<&backend::WorkItem>,
) -> String {
    let mut sections = Vec::new();

    let artifact_markers = artifacts
        .iter()
        .filter_map(|artifact| {
            let content_excerpt = artifact.content.as_ref().map(|content| {
                content
                    .split_whitespace()
                    .take(48)
                    .collect::<Vec<_>>()
                    .join(" ")
            });
            serde_json::to_string(&ArtifactReferencePayload {
                id: artifact.id,
                label: artifact.label.clone(),
                kind: artifact.kind.clone(),
                domain: artifact.domain.clone(),
                file_path: artifact.file_path.clone(),
                content_type: artifact.content_type.clone(),
                content_excerpt,
            })
            .ok()
            .map(|json| format!("[[ARTIFACT_REF:{}]]", json))
        })
        .collect::<Vec<_>>();

    if !artifact_markers.is_empty() {
        sections.push(format!(
            "ARTIFACT_CONTEXT:\n{}",
            artifact_markers.join("\n")
        ));
    }

    if let Some(work_item) = work_item {
        if let Ok(json) = serde_json::to_string(&WorkItemReferencePayload {
            id: work_item.id,
            title: work_item.title.clone(),
            brief: work_item.brief.clone(),
            status: work_item.status.clone(),
            surface_kind: work_item.surface_kind.clone(),
            assigned_domain: work_item.assigned_domain.clone(),
            prompt: work_item.prompt.clone(),
        }) {
            sections.push(format!("WORK_ITEM_CONTEXT:\n[[WORK_ITEM_REF:{}]]", json));
        }
    }

    if sections.is_empty() {
        query.to_string()
    } else {
        format!("{}\n\n{}", query.trim(), sections.join("\n\n"))
    }
}

pub fn parse_message(content: &str) -> ParsedMessage {
    let mut attachments = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(inner) = trimmed.strip_prefix("[[ATTACH_FILE:")
            .and_then(|s| s.strip_suffix("]]"))
        {
            attachments.push(inner.to_string());
        } else if let Some(inner) = trimmed.strip_prefix("[[ATTACH_IMAGE:")
            .and_then(|s| s.strip_suffix("]]"))
        {
            attachments.push(inner.to_string());
        }
    }

    ParsedMessage {
        display_text: clean_content(content),
        attachments,
    }
}

pub fn extract_agent_attachments(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("[[ATTACH_FILE:")
                .and_then(|s| s.strip_suffix("]]"))
                .map(|path| path.to_string())
        })
        .collect()
}

pub fn clean_content(content: &str) -> String {
    let without_attach_markers = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed == "ARTIFACT_CONTEXT:" || trimmed == "WORK_ITEM_CONTEXT:" {
                return false;
            }
            // Strip any line that contains an internal attachment or reference marker
            !(trimmed.contains("[[ATTACH_IMAGE:")
                || trimmed.contains("[[ATTACH_FILE:")
                || trimmed.contains("[[ARTIFACT_REF:")
                || trimmed.contains("[[WORK_ITEM_REF:"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let trimmed = without_attach_markers.trim();

    // 1. Reject raw tool calls — agent sent JSON instead of a final answer
    if trimmed.starts_with('{') && trimmed.contains("\"tool\"") {
        return "Agent sent a tool call incorrectly. Check logs.".to_string();
    }

    // 2. Handle ---CHART_XXX--- markers produced by the charting tool.
    //    The charting tool always returns a clean marker on its own line.
    //    We only need to clean up any stray separator lines that appear
    //    before the marker in the conversation history.
    if let Some(pos) = trimmed.find("---CHART_") {
        let before = trimmed[..pos]
            .trim()
            .trim_end_matches("---")
            .trim_end_matches("--")
            .trim();

        let from_marker = &trimmed[pos..];
        let first_newline = from_marker.find('\n').unwrap_or(from_marker.len());
        let marker_line = from_marker[..first_newline].trim();

        // The payload follows the marker line — strip any code fence the model added
        let rest = from_marker[first_newline..].trim();
        let inner = if rest.starts_with("```") {
            let after_fence = rest.lines().skip(1).collect::<Vec<_>>().join("\n");
            if let Some(end) = after_fence.rfind("```") {
                after_fence[..end].trim().to_string()
            } else {
                after_fence.trim().to_string()
            }
        } else {
            // Strip a trailing --- separator if present
            if let Some(end) = rest.rfind("\n---\n").or_else(|| rest.rfind("\n---")) {
                rest[..end].trim().to_string()
            } else {
                rest.trim().to_string()
            }
        };

        return if before.is_empty() {
            format!("{}\n{}", marker_line, inner)
        } else {
            format!("{}\n{}\n{}", before, marker_line, inner)
        };
    }

    // 3. Everything else is plain text — return as-is.
    //    The dispatcher guarantees that charts are returned directly from the
    //    charting tool as a clean ---CHART_SVG--- block, never mixed with prose.
    trimmed.to_string()
}

pub fn copy_to_clipboard(text: String) {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    let eval_str = format!("navigator.clipboard.writeText('{}')", escaped);
    let _ = eval(&eval_str);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_marker_no_text_before() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" baseProfile="full" viewBox="0 0 800 500"><rect width="800" height="500"/></svg>"#;
        let content = format!("---CHART_SVG---\n{}", svg);
        let cleaned = clean_content(&content);
        assert!(cleaned.starts_with("---CHART_SVG---"));
        assert!(cleaned.contains("<svg"));
    }

    #[test]
    fn test_chart_marker_with_text_before() {
        let svg =
            r#"<svg xmlns="http://www.w3.org/2000/svg" baseProfile="full" viewBox="0 0 800 500"/>"#;
        let content = format!(
            "Next.js grew from 1.8% to 3.0%.\n\n---\n---CHART_SVG---\n{}",
            svg
        );
        let cleaned = clean_content(&content);
        assert!(cleaned.starts_with("Next.js grew"));
        assert!(cleaned.contains("---CHART_SVG---"));
        // Stray separator dashes should be stripped
        assert!(!cleaned.contains("\n---\n---CHART"));
    }

    #[test]
    fn test_plain_text_passthrough() {
        let content = "Here is some analysis about Next.js growth.";
        let cleaned = clean_content(content);
        assert_eq!(cleaned, content);
    }

    #[test]
    fn test_attach_markers_are_hidden_from_chat_text() {
        let content = "Here is the screenshot again.\n[[ATTACH_IMAGE:C:\\temp\\desktop.png]]";
        let cleaned = clean_content(content);
        assert_eq!(cleaned, "Here is the screenshot again.");
    }

    #[test]
    fn test_artifact_ref_markers_are_hidden_from_chat_text() {
        let content = "Use this file.\n[[ARTIFACT_REF:{\"id\":1,\"label\":\"Spec\"}]]";
        let cleaned = clean_content(content);
        assert_eq!(cleaned, "Use this file.");
    }

    #[test]
    fn test_work_item_markers_are_hidden_from_chat_text() {
        let content =
            "Do this next.\nWORK_ITEM_CONTEXT:\n[[WORK_ITEM_REF:{\"id\":1,\"title\":\"Review\"}]]";
        let cleaned = clean_content(content);
        assert_eq!(cleaned, "Do this next.");
    }

    #[test]
    fn test_tool_call_rejected() {
        let content = r#"{"tool": "charting", "input": {}}"#;
        let cleaned = clean_content(content);
        assert!(cleaned.contains("Agent sent a tool call incorrectly"));
    }

    #[test]
    fn test_bulleted_attach_markers_are_hidden() {
        let content = "**Artifacts**:\n- [[ATTACH_FILE:assets/exports/report.csv]]";
        let cleaned = clean_content(content);
        assert_eq!(cleaned, "**Artifacts**:");
    }

    #[test]
    fn test_no_svg_fallback() {
        // Raw SVG in the message must NOT be extracted — plain text passthrough only
        let content = r#"Here is a chart: <svg width="800"><rect/></svg> end."#;
        let cleaned = clean_content(content);
        assert!(!cleaned.contains("---CHART_SVG---"));
        assert_eq!(cleaned, content.trim());
    }
}
