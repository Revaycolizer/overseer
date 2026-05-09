// src/routes/conversations/artifact_workbench_parts.rs
use super::artifact_workbench_utils::CreateWorkItemButton;
use super::helpers::copy_to_clipboard;
use super::route_summary::prettify_token;
use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn WorkbenchSurfaces(
    active_surface: Signal<String>,
    has_other_selected: bool,
    artifact: backend::Artifact,
    other_selected_labels: Vec<String>,
    starter_prompt: Signal<Option<String>>,
    db: std::sync::Arc<crate::db::Database>,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
) -> Element {
    rsx! {
        div { class: "space-y-3",
            div { class: "flex flex-wrap gap-2",
                ArtifactSurfaceModeButton { label: "Summary".to_string(), icon: "summarize".to_string(), value: "summary".to_string(), active_surface: active_surface }
                ArtifactSurfaceModeButton { label: "Review".to_string(), icon: "fact_check".to_string(), value: "review".to_string(), active_surface: active_surface }
                if has_other_selected { ArtifactSurfaceModeButton { label: "Compare".to_string(), icon: "compare".to_string(), value: "compare".to_string(), active_surface: active_surface } }
                ArtifactSurfaceModeButton { label: "Handoff".to_string(), icon: "forward_to_inbox".to_string(), value: "handoff".to_string(), active_surface: active_surface }
            }
            ArtifactTaskSurface { mode: active_surface(), artifact: artifact.clone(), other_selected_labels: other_selected_labels, starter_prompt: starter_prompt }
            CreateWorkItemButton { db: db, artifact: artifact, active_surface: active_surface, selected_artifacts: selected_artifacts, selected_work_item: selected_work_item, work_items_resource: work_items_resource, work_item_events_resource: work_item_events_resource }
        }
    }
}

#[component]
fn ArtifactSurfaceModeButton(label: String, icon: String, value: String, active_surface: Signal<String>) -> Element {
    let is_active = active_surface() == value;
    rsx! {
        button {
            class: format!("inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] transition-all {}", if is_active { "border-blue-200 bg-blue-50 text-blue-700" } else { "border-slate-200 bg-white text-slate-600 hover:border-blue-200 hover:text-blue-700" }),
            onclick: move |_| active_surface.set(value.clone()),
            MaterialIcon { name: icon, size: 14 }
            "{label}"
        }
    }
}

#[component]
fn ArtifactTaskSurface(mode: String, artifact: backend::Artifact, other_selected_labels: Vec<String>, starter_prompt: Signal<Option<String>>) -> Element {
    let (title, description, prompt, action_label, icon) = artifact_surface_definition(&mode, &artifact.kind);
    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-white px-4 py-4 shadow-sm space-y-3",
            div {
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "{title}" }
                p { class: "text-xs text-slate-600 mt-2 leading-relaxed", "{description}" }
            }
            if mode == "compare" {
                if other_selected_labels.is_empty() {
                    div { class: "rounded-xl border border-dashed border-slate-200 bg-slate-50 px-3 py-3 text-xs text-slate-500", "Select at least one more artifact to unlock a real compare surface." }
                } else {
                    div { class: "space-y-2",
                        p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Also Selected" }
                        div { class: "flex flex-wrap gap-2",
                            for label in other_selected_labels { span { class: "inline-flex items-center rounded-full border border-slate-200 bg-slate-50 px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.14em] text-slate-600", "{label}" } }
                        }
                    }
                }
            } else if let Some(path) = artifact.file_path.clone().filter(|v| !v.trim().is_empty()) {
                div { class: "space-y-2",
                    p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Working From" }
                    p { class: "text-[11px] font-mono text-slate-500 break-all", "{path}" }
                }
            } else if let Some(content) = artifact.content.clone().filter(|v| !v.trim().is_empty()) {
                div { class: "space-y-2",
                    p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Working Excerpt" }
                    p { class: "text-xs text-slate-600 whitespace-pre-wrap break-words line-clamp-4", "{content}" }
                }
            }
            div { class: "rounded-xl border border-slate-200 bg-slate-50 px-3 py-3",
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Task Prompt" }
                p { class: "text-xs text-slate-700 mt-2 leading-relaxed", "{prompt}" }
            }
            button {
                class: "inline-flex items-center gap-2 rounded-xl border border-blue-200 bg-blue-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-blue-700 hover:bg-blue-100 transition-all",
                onclick: { let prompt = prompt.to_string(); move |_| starter_prompt.set(Some(prompt.clone())) },
                MaterialIcon { name: icon, size: 14 }
                "{action_label}"
            }
        }
    }
}

pub fn artifact_surface_definition<'a>(mode: &'a str, artifact_kind: &str) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
    match mode {
        "review" => ("Review Surface", "Use this when you want a grounded read on what matters, what looks wrong, or what deserves attention in the artifact.",
            match artifact_kind { "image" => "Review the selected screenshot carefully and call out important details, issues, or anomalies.", "file" => "Review the selected file and identify the most important details, risks, or follow-up questions.", _ => "Review the selected artifact and tell me what stands out, what matters, and what needs attention." },
            "Start Review", "fact_check"),
        "compare" => ("Compare Surface", "Turn the selected artifacts into a comparison task instead of relying on a generic follow-up in chat.", "Compare the selected artifacts and explain the important differences, shared patterns, and best next step.", "Compare Artifacts", "compare"),
        "handoff" => ("Handoff Surface", "Package this artifact into a reusable handoff for the next agent, workflow, or human reviewer.", "Turn the selected artifact into a clean handoff with context, key details, and the recommended next action.", "Create Handoff", "forward_to_inbox"),
        _ => ("Summary Surface", "Condense the artifact into the shortest useful understanding before you branch into deeper work.", "Summarize the selected artifact and tell me what matters most.", "Create Summary", "summarize"),
    }
}
