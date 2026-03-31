use crate::backend;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct ArtifactReferencePayload { id: i64, label: String, kind: String, domain: Option<String>, file_path: Option<String>, content_type: Option<String>, content_excerpt: Option<String> }

#[derive(Debug, Clone, Serialize)]
struct WorkItemReferencePayload { id: i64, title: String, brief: Option<String>, status: String, surface_kind: String, assigned_domain: Option<String>, prompt: String }

pub fn build_query_with_context(query: &str, artifacts: &[backend::Artifact], work_item: Option<&backend::WorkItem>) -> String {
    let mut sections = Vec::new();
    let artist_markers = artifacts.iter().filter_map(|artifact| {
        let content_excerpt = artifact.content.as_ref().map(|content| content.split_whitespace().take(48).collect::<Vec<_>>().join(" "));
        serde_json::to_string(&ArtifactReferencePayload { id: artifact.id, label: artifact.label.clone(), kind: artifact.kind.clone(), domain: artifact.domain.clone(), file_path: artifact.file_path.clone(), content_type: artifact.content_type.clone(), content_excerpt }).ok().map(|json| format!("[[ARTIFACT_REF:{}]]", json))
    }).collect::<Vec<_>>();
    
    if !artist_markers.is_empty() { sections.push(format!("ARTIFACT_CONTEXT:\n{}", artist_markers.join("\n"))); }
    if let Some(work_item) = work_item {
        if let Ok(json) = serde_json::to_string(&WorkItemReferencePayload { id: work_item.id, title: work_item.title.clone(), brief: work_item.brief.clone(), status: work_item.status.clone(), surface_kind: work_item.surface_kind.clone(), assigned_domain: work_item.assigned_domain.clone(), prompt: work_item.prompt.clone() }) {
            sections.push(format!("WORK_ITEM_CONTEXT:\n[[WORK_ITEM_REF:{}]]", json));
        }
    }
    if sections.is_empty() { query.to_string() } else { format!("{}\n\n{}", query.trim(), sections.join("\n\n")) }
}
