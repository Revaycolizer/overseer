// src/routes/conversations/workspace_deck.rs
use super::artifact_workbench::ArtifactWorkbenchPanel;
use super::work_item_panel::WorkItemWorkspacePanel;
use super::workspace_session::{workspace_session_key, WorkspaceSessionRef};
use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn WorkspaceSessionDeck(
    db: std::sync::Arc<crate::db::Database>,
    agent_system: std::sync::Arc<crate::engine::system::AgentSystem>,
    messages_resource: Resource<Vec<backend::Message>>,
    visible_messages: Signal<Vec<backend::Message>>,
    is_thinking: Signal<bool>,
    sessions: Vec<WorkspaceSessionRef>,
    sessions_signal: Signal<Vec<WorkspaceSessionRef>>,
    active_session_key: Signal<Option<String>>,
    artifacts: Vec<backend::Artifact>,
    work_items: Vec<backend::WorkItem>,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    execution_runs_resource: Resource<Vec<backend::ExecutionRun>>,
    artifacts_resource: Resource<Vec<backend::Artifact>>,
    work_item_events: Vec<backend::WorkItemEvent>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
    selected_execution_run_id: Signal<Option<i64>>,
    on_open: EventHandler<String>,
) -> Element {
    let active_key = active_session_key().or_else(|| sessions.first().map(workspace_session_key));
    let active_session = active_key.as_ref().and_then(|key| sessions.iter().find(|s| workspace_session_key(s) == *key).cloned());

    rsx! {
        div { class: "space-y-4",
            div { class: "flex items-center justify-between gap-3",
                div {
                    p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "Workspace Sessions" }
                    p { class: "text-xs text-slate-500 mt-1", "Keep artifact and work-item contexts open as switchable task surfaces." }
                }
                span { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "{sessions.len()} open" }
            }
            div { class: "flex flex-wrap gap-2",
                for session in sessions.iter().cloned() {
                    WorkspaceSessionTab { session: session, sessions_signal: sessions_signal, active_session_key: active_session_key, active_key: active_key.clone(), sessions: sessions.clone(), artifacts: artifacts.clone(), work_items: work_items.clone() }
                }
            }
            if let Some(session) = active_session {
                match session {
                    WorkspaceSessionRef::Artifact(artifact_id) => {
                        if let Some(artifact) = artifacts.iter().find(|a| a.id == artifact_id).cloned() {
                            rsx! { ArtifactWorkbenchPanel { db: db, artifact: artifact, selected_artifacts: selected_artifacts, selected_work_item: selected_work_item, starter_prompt: starter_prompt, work_items_resource: work_items_resource, work_item_events_resource: work_item_events_resource.clone(), on_open: on_open } }
                        } else {
                            rsx! { div { class: "rounded-2xl border border-dashed border-slate-200 bg-white px-4 py-5 text-sm text-slate-500", "This artifact is no longer available in the current conversation state." } }
                        }
                    }
                    WorkspaceSessionRef::WorkItem(work_item_id) => {
                        if let Some(work_item) = work_items.iter().find(|w| w.id == work_item_id).cloned() {
                            rsx! { WorkItemWorkspacePanel { db: db.clone(), agent_system: agent_system.clone(), work_item: work_item, messages_resource: messages_resource.clone(), visible_messages: visible_messages, selected_work_item: selected_work_item, is_thinking: is_thinking, starter_prompt: starter_prompt, work_items_resource: work_items_resource.clone(), execution_runs_resource: execution_runs_resource.clone(), artifacts_resource: artifacts_resource.clone(), work_item_events: work_item_events.clone(), work_item_events_resource: work_item_events_resource.clone(), selected_execution_run_id: selected_execution_run_id } }
                        } else {
                            rsx! { div { class: "rounded-2xl border border-dashed border-slate-200 bg-white px-4 py-5 text-sm text-slate-500", "This work item is no longer available in the current conversation state." } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn WorkspaceSessionTab(
    session: WorkspaceSessionRef, sessions_signal: Signal<Vec<WorkspaceSessionRef>>,
    active_session_key: Signal<Option<String>>, active_key: Option<String>,
    sessions: Vec<WorkspaceSessionRef>, artifacts: Vec<backend::Artifact>, work_items: Vec<backend::WorkItem>,
) -> Element {
    let session_key = workspace_session_key(&session);
    let is_active = active_key.as_ref().is_some_and(|k| k == &session_key);
    let label = match session {
        WorkspaceSessionRef::Artifact(id) => artifacts.iter().find(|a| a.id == id).map(|a| a.label.clone()).unwrap_or_else(|| format!("Artifact {id}")),
        WorkspaceSessionRef::WorkItem(id) => work_items.iter().find(|w| w.id == id).map(|w| w.title.clone()).unwrap_or_else(|| format!("Work Item {id}")),
    };
    rsx! {
        div { class: format!("inline-flex items-center gap-2 rounded-2xl border px-3 py-2 transition-all {}", if is_active { "border-blue-200 bg-blue-50 text-blue-700" } else { "border-slate-200 bg-white text-slate-700" }),
            button { class: "text-[10px] font-black uppercase tracking-[0.14em]",
                onclick: { let session_key = session_key.clone(); move |_| active_session_key.set(Some(session_key.clone())) }, "{label}" }
            button { class: "inline-flex size-5 items-center justify-center rounded-full text-slate-400 hover:bg-slate-100 hover:text-slate-700 transition-all",
                onclick: { let session_key = session_key.clone(); let sessions = sessions.clone();
                    move |_| {
                        let next_key = sessions.iter().filter(|e| workspace_session_key(e) != session_key).map(workspace_session_key).next();
                        sessions_signal.with_mut(|s| s.retain(|e| workspace_session_key(e) != session_key));
                        active_session_key.with_mut(|a| { if a.as_ref().is_some_and(|c| c == &session_key) { *a = next_key; } });
                    }
                },
                MaterialIcon { name: "close", size: 14 }
            }
        }
    }
}
