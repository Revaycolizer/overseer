use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactReference {
    pub id: i64, pub label: String, pub kind: String,
    pub domain: Option<String>, pub file_path: Option<String>,
    pub content_type: Option<String>, pub content_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemReference {
    pub id: i64, pub title: String, pub brief: Option<String>,
    pub status: String, pub surface_kind: String,
    pub assigned_domain: Option<String>, pub prompt: String,
}

pub fn extract_structured_context(query: &str) -> (String, Vec<ArtifactReference>, Vec<WorkItemReference>) {
    let mut refs = Vec::new();
    let mut work_items = Vec::new();
    let mut visible_lines = Vec::new();

    for line in query.lines() {
        let t = line.trim();
        if t == "ARTIFACT_CONTEXT:" || t == "WORK_ITEM_CONTEXT:" { continue; }
        if let Some(p) = t.strip_prefix("[[ARTIFACT_REF:").and_then(|v| v.strip_suffix("]]")) {
            if let Ok(parsed) = serde_json::from_str::<ArtifactReference>(p) { refs.push(parsed); }
            continue;
        }
        if let Some(p) = t.strip_prefix("[[WORK_ITEM_REF:").and_then(|v| v.strip_suffix("]]")) {
            if let Ok(parsed) = serde_json::from_str::<WorkItemReference>(p) { work_items.push(parsed); }
            continue;
        }
        visible_lines.push(line);
    }
    (visible_lines.join("\n").trim().to_string(), refs, work_items)
}

pub fn format_artifact_context(artifact_refs: &[ArtifactReference]) -> String {
    if artifact_refs.is_empty() { return String::new(); }
    let lines = artifact_refs.iter().map(|a| {
        let mut p = vec![format!("Artifact #{}", a.id), format!("label={}", a.label), format!("kind={}", a.kind)];
        if let Some(d) = a.domain.as_deref().filter(|v| !v.trim().is_empty()) { p.push(format!("domain={d}")); }
        if let Some(path) = a.file_path.as_deref().filter(|v| !v.trim().is_empty()) { p.push(format!("path={path}")); }
        if let Some(ct) = a.content_type.as_deref().filter(|v| !v.trim().is_empty()) { p.push(format!("content_type={ct}")); }
        if let Some(ex) = a.content_excerpt.as_deref().filter(|v| !v.trim().is_empty()) { p.push(format!("excerpt={ex}")); }
        format!("- {}", p.join(" | "))
    }).collect::<Vec<_>>().join("\n");
    format!("SELECTED ARTIFACTS:\n{}", lines)
}

pub fn format_work_item_context(work_items: &[WorkItemReference]) -> String {
    if work_items.is_empty() { return String::new(); }
    let lines = work_items.iter().map(|w| {
        let mut p = vec![format!("WorkItem #{}", w.id), format!("title={}", w.title), format!("status={}", w.status), format!("surface={}", w.surface_kind), format!("prompt={}", w.prompt)];
        if let Some(d) = w.assigned_domain.as_deref().filter(|v| !v.trim().is_empty()) { p.push(format!("assigned_domain={d}")); }
        if let Some(b) = w.brief.as_deref().filter(|v| !v.trim().is_empty()) { p.push(format!("brief={b}")); }
        format!("- {}", p.join(" | "))
    }).collect::<Vec<_>>().join("\n");
    format!("ACTIVE WORK ITEMS:\n{}", lines)
}
