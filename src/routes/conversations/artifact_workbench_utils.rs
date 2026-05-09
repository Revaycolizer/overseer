// src/routes/conversations/artifact_workbench_utils.rs
use super::artifact_workbench_parts::artifact_surface_definition;
use super::helpers::copy_to_clipboard;
use super::route_summary::prettify_token;
use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn CreateWorkItemButton(
    db: std::sync::Arc<crate::db::Database>,
    artifact: backend::Artifact,
    active_surface: Signal<String>,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
) -> Element {
    rsx! {
        button {
            class: "inline-flex items-center gap-2 rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-emerald-700 hover:bg-emerald-100 transition-all",
            onclick: {
                let db = db.clone(); let artifact = artifact.clone();
                let selected_artifacts = selected_artifacts; let mut selected_work_item = selected_work_item;
                let mut work_items_resource = work_items_resource.clone();
                move |_| {
                    let db = db.clone(); let artifact = artifact.clone();
                    let selected_artifacts = selected_artifacts; let mut selected_work_item = selected_work_item;
                    let mut work_items_resource = work_items_resource.clone();
                    let active_mode = active_surface();
                    spawn(async move {
                        let (_, brief, prompt, _, _) = artifact_surface_definition(&active_mode, &artifact.kind);
                        let source_artifact_ids = selected_artifacts().iter().map(|i| i.id).collect::<Vec<_>>();
                        let source_artifact_ids_json = serde_json::to_string(&source_artifact_ids).ok();
                        let title = format!("{}: {}", prettify_token(&active_mode), artifact.label);
                        if let Ok(id) = backend::create_work_item(&db, artifact.conversation_id, &title, Some(brief), "queued", &active_mode, artifact.domain.as_deref(), prompt, source_artifact_ids_json.as_deref(), None).await {
                            let _ = backend::log_work_item_event(&db, id, None, "created", Some("queued"), Some("Work item created from an artifact surface."), None).await;
                            let now = chrono::Utc::now().naive_utc();
                            selected_work_item.set(Some(backend::WorkItem { id, conversation_id: artifact.conversation_id, title, brief: Some(brief.to_string()), status: "queued".to_string(), surface_kind: active_mode, assigned_domain: artifact.domain.clone(), prompt: prompt.to_string(), source_artifact_ids_json, metadata_json: None, created_at: now, updated_at: now }));
                            work_items_resource.restart();
                            work_item_events_resource.restart();
                        }
                    });
                }
            },
            MaterialIcon { name: "add_task", size: 14 }
            "Create Work Item"
        }
    }
}

#[component]
pub fn WorkbenchUtilities(
    artifact: backend::Artifact,
    is_selected: bool,
    has_other_selected: bool,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    starter_prompt: Signal<Option<String>>,
    on_open: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "space-y-2",
            p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Utilities" }
            div { class: "flex flex-wrap gap-2",
                button { class: format!("inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] transition-all {}", if is_selected { "border-blue-200 bg-blue-50 text-blue-700" } else { "border-slate-200 bg-white text-slate-700 hover:border-blue-200 hover:text-blue-700" }),
                    onclick: { let artifact = artifact.clone(); move |_| { selected_artifacts.with_mut(|s| { if let Some(i) = s.iter().position(|x| x.id == artifact.id) { s.remove(i); } else { s.push(artifact.clone()); } }); } },
                    MaterialIcon { name: if is_selected { "task_alt" } else { "add_circle" }, size: 14 }
                    if is_selected { "Selected for Prompt" } else { "Use in Next Prompt" }
                }
                button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                    onclick: move |_| starter_prompt.set(Some("Summarize the selected artifact and tell me what matters most.".to_string())),
                    MaterialIcon { name: "summarize", size: 14 } "Summarize" }
                if artifact.kind == "image" {
                    button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: move |_| starter_prompt.set(Some("Analyze the selected screenshot and describe what is visible, important, or unusual.".to_string())),
                        MaterialIcon { name: "image_search", size: 14 } "Analyze Screenshot" }
                }
                if artifact.kind == "file" {
                    button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: move |_| starter_prompt.set(Some("Read the selected file and summarize its contents.".to_string())),
                        MaterialIcon { name: "description", size: 14 } "Read File" }
                }
                if has_other_selected {
                    button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: move |_| starter_prompt.set(Some("Compare the selected artifacts and explain the important differences.".to_string())),
                        MaterialIcon { name: "compare", size: 14 } "Compare Selected" }
                }
                if let Some(path) = artifact.file_path.clone().filter(|v| !v.trim().is_empty()) {
                    button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: { let path = path.clone(); move |_| on_open.call(path.clone()) }, MaterialIcon { name: "open_in_new", size: 14 } "Open" }
                    button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: { let path = path.clone(); move |_| copy_to_clipboard(path.clone()) }, MaterialIcon { name: "content_copy", size: 14 } "Copy Path" }
                } else if let Some(content) = artifact.content.clone().filter(|v| !v.trim().is_empty()) {
                    button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: { let content = content.clone(); move |_| copy_to_clipboard(content.clone()) }, MaterialIcon { name: "content_copy", size: 14 } "Copy Content" }
                }
            }
        }
    }
}
