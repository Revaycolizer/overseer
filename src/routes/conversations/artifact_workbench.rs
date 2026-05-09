// src/routes/conversations/artifact_workbench.rs
use super::artifact_workbench_parts::WorkbenchSurfaces;
use super::artifact_workbench_utils::WorkbenchUtilities;
use super::artifact_panel::ArtifactKindBadge;
use super::helpers::copy_to_clipboard;
use super::route_summary::prettify_token;
use base64::{engine::general_purpose, Engine as _};
use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn ArtifactWorkbenchPanel(
    db: std::sync::Arc<crate::db::Database>,
    artifact: backend::Artifact,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
    on_open: EventHandler<String>,
) -> Element {
    let artifact_kind = artifact.kind.clone();
    let artifact_content_type = artifact.content_type.clone();
    let file_path = artifact.file_path.clone();
    let image_preview = use_resource(move || {
        let kind = artifact_kind.clone(); let file_path = file_path.clone();
        let content_type = artifact_content_type.clone().unwrap_or_else(|| "image/png".to_string());
        async move {
            if kind != "image" { return None; }
            let Some(path) = file_path else { return None; };
            match std::fs::read(&path) {
                Ok(bytes) if !bytes.is_empty() => Some(format!("data:{};base64,{}", content_type, general_purpose::STANDARD.encode(bytes))),
                _ => None,
            }
        }
    });
    let is_selected = selected_artifacts().iter().any(|s| s.id == artifact.id);
    let has_other_selected = selected_artifacts().iter().any(|s| s.id != artifact.id);
    let other_selected_labels = selected_artifacts().iter().filter(|s| s.id != artifact.id).map(|s| s.label.clone()).collect::<Vec<_>>();
    let default_surface = if has_other_selected { "compare".to_string() } else { "summary".to_string() };
    let mut active_surface = use_signal(move || default_surface.clone());

    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 to-white px-4 py-4 shadow-sm space-y-4",
            div {
                p { class: "text-[10px] font-black uppercase tracking-[0.2em] text-slate-400", "Artifact Inspector" }
                h4 { class: "text-sm font-black text-slate-900 mt-1", "{artifact.label}" }
                p { class: "text-xs text-slate-500 mt-1 leading-relaxed", "Inspect this artifact, reuse it in the next prompt, or turn it into a more targeted task." }
            }
            div { class: "flex flex-wrap items-center gap-2",
                ArtifactKindBadge { kind: artifact.kind.clone() }
                if let Some(domain) = artifact.domain.clone().filter(|v| !v.trim().is_empty()) {
                    span { class: "inline-flex items-center rounded-full border border-slate-200 bg-white px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500", "{prettify_token(&domain)}" }
                }
                span { class: "text-[10px] text-slate-400", {artifact.created_at.format("%Y-%m-%d %H:%M").to_string()} }
            }
            if artifact.kind == "image" {
                match image_preview().flatten() {
                    Some(src) => rsx! { img { src: "{src}", alt: "{artifact.label}", class: "rounded-xl border border-slate-200 bg-white shadow-sm max-w-full h-auto block" } },
                    None => rsx! { div { class: "rounded-xl border border-dashed border-slate-200 bg-white px-4 py-5 text-xs text-slate-400", "Image preview unavailable." } },
                }
            }
            if let Some(step_text) = artifact.step_text.clone().filter(|v| !v.trim().is_empty()) {
                p { class: "text-xs text-slate-600 leading-relaxed", "{step_text}" }
            }
            WorkbenchSurfaces { active_surface: active_surface, has_other_selected: has_other_selected, artifact: artifact.clone(), other_selected_labels: other_selected_labels.clone(), starter_prompt: starter_prompt, db: db, selected_artifacts: selected_artifacts, selected_work_item: selected_work_item, work_items_resource: work_items_resource, work_item_events_resource: work_item_events_resource }
            WorkbenchUtilities { artifact: artifact, is_selected: is_selected, has_other_selected: has_other_selected, selected_artifacts: selected_artifacts, starter_prompt: starter_prompt, on_open: on_open }
        }
    }
}
