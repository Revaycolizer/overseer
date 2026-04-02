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

pub fn format_newly_created_artifacts(artifacts: &[String], step_description: &str) -> String {
    if artifacts.is_empty() { return String::new(); }
    let mut lines = Vec::new();
    for a in artifacts {
        let (kind, ct) = crate::engine::orchestrator::utils::artifact_utils::infer_artifact_kind_and_content_type(a);
        let label = crate::engine::orchestrator::utils::artifact_utils::infer_artifact_label(&kind, step_description, a);
        
        let mut p = vec![format!("Artifact [NEW]"), format!("label={}", label), format!("kind={}", kind)];
        if let Some(c) = ct { p.push(format!("content_type={}", c)); }
        p.push(format!("path={}", a));
        
        lines.push(format!("- {}", p.join(" | ")));
    }
    format!("NEWLY CREATED ARTIFACTS:\n{}", lines.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"))
}

pub fn format_step_results_context(step_description: &str, summary: &str, evidence: &[String]) -> String {
    let t_summary = summary.trim();
    
    // Check if it's just a generic completion message
    let lower = t_summary.to_lowercase();
    let is_generic = t_summary.is_empty() || lower == "task completed." || lower == "done." || lower == "finished." || lower == "success.";
    
    if is_generic && evidence.is_empty() {
        return String::new();
    }

    let mut findings_content = String::new();
    if !is_generic {
        findings_content.push_str(t_summary);
    }
    
    if !evidence.is_empty() {
        if !findings_content.is_empty() { findings_content.push_str("\n- ADDITIONAL_EVIDENCE: "); }
        let readable_evidence: Vec<String> = evidence.iter().map(|e| {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(e) {
                let title = v.get("title").or(v.get("source")).and_then(|v| v.as_str()).unwrap_or("Evidence");
                let snippet = v.get("snippet").or(v.get("description")).and_then(|v| v.as_str());
                if let Some(s) = snippet { format!("{}: {}", title, s) } else { title.to_string() }
            } else {
                e.clone()
            }
        }).collect();
        findings_content.push_str(&readable_evidence.join("; "));
    }

    format!("KNOWLEDGE_FROM_PREVIOUS_STEPS:\n- STEP: {}\n- FINDINGS: {}\n", step_description, findings_content)
}
