// src/routes/conversations/work_item_panel_parts.rs
use super::helpers::copy_to_clipboard;
use super::route_summary::prettify_token;
use super::work_item_card::WorkItemStatusBadge;
use super::work_item_handoff::create_work_item_handoff;
use super::work_item_ops::parse_work_item_runtime_metadata;
use super::work_item_runner::{run_work_item_now, update_work_item_manual_status};
use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn WorkItemPanelHeader(work_item: backend::WorkItem, runtime_metadata_excerpt: Option<String>, runtime_metadata_status: Option<String>, runtime_metadata_run_id: Option<i64>, runtime_metadata_executed_at: Option<String>, latest_artifact_count: usize) -> Element {
    rsx! {
        div {
            p { class: "text-[10px] font-black uppercase tracking-[0.2em] text-slate-400", "Work Item Session" }
            h4 { class: "text-sm font-black text-slate-900 mt-1", "{work_item.title}" }
            if let Some(brief) = work_item.brief.clone().filter(|v| !v.trim().is_empty()) { p { class: "text-xs text-slate-500 mt-1 leading-relaxed", "{brief}" } }
        }
        div { class: "flex flex-wrap items-center gap-2",
            WorkItemStatusBadge { status: work_item.status.clone() }
            span { class: "inline-flex items-center rounded-full border border-slate-200 bg-white px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500", "{prettify_token(&work_item.surface_kind)}" }
            if let Some(domain) = work_item.assigned_domain.clone().filter(|v| !v.trim().is_empty()) { span { class: "inline-flex items-center rounded-full border border-slate-200 bg-white px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500", "{prettify_token(&domain)}" } }
        }
        div { class: "rounded-xl border border-slate-200 bg-white px-4 py-3 space-y-2",
            p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Prompt" }
            p { class: "text-sm text-slate-700 whitespace-pre-wrap break-words", "{work_item.prompt}" }
        }
        if let Some(excerpt) = runtime_metadata_excerpt.filter(|v| !v.trim().is_empty()) {
            div { class: "rounded-xl border border-slate-200 bg-white px-4 py-3 space-y-2",
                div { class: "flex flex-wrap items-center justify-between gap-2",
                    p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Latest Result" }
                    if let Some(status) = runtime_metadata_status.filter(|v| !v.trim().is_empty()) { span { class: "text-[10px] font-black uppercase tracking-[0.16em] text-slate-500", "{prettify_token(&status)}" } }
                }
                p { class: "text-sm text-slate-700 whitespace-pre-wrap break-words", "{excerpt}" }
                div { class: "flex flex-wrap items-center gap-2 text-[10px] font-semibold text-slate-400",
                    if let Some(run_id) = runtime_metadata_run_id { span { "Run #{run_id}" } }
                    if latest_artifact_count > 0 { span { "{latest_artifact_count} artifact(s)" } }
                    if let Some(last) = runtime_metadata_executed_at.filter(|v| !v.trim().is_empty()) { span { "{last}" } }
                }
            }
        }
    }
}

#[component]
pub fn WorkItemPanelActions(db: std::sync::Arc<crate::db::Database>, agent_system: std::sync::Arc<crate::engine::system::AgentSystem>, work_item: backend::WorkItem, messages_resource: Resource<Vec<backend::Message>>, visible_messages: Signal<Vec<backend::Message>>, selected_work_item: Signal<Option<backend::WorkItem>>, is_thinking: Signal<bool>, starter_prompt: Signal<Option<String>>, can_retry: bool, work_items_resource: Resource<Vec<backend::WorkItem>>, execution_runs_resource: Resource<Vec<backend::ExecutionRun>>, artifacts_resource: Resource<Vec<backend::Artifact>>, work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>, selected_execution_run_id: Signal<Option<i64>>) -> Element {
    rsx! {
        div { class: "flex flex-wrap gap-2",
            button { class: "inline-flex items-center gap-2 rounded-xl border border-violet-200 bg-violet-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-violet-700 hover:bg-violet-100 transition-all",
                onclick: { let work_item = work_item.clone(); move |_| { selected_work_item.set(Some(work_item.clone())); starter_prompt.set(Some(work_item.prompt.clone())); } },
                MaterialIcon { name: "edit_note", size: 14 } "Use in Composer" }
            button { class: format!("inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] transition-all {}", if is_thinking() { "border-slate-200 bg-slate-100 text-slate-400 cursor-not-allowed" } else { "border-blue-200 bg-blue-50 text-blue-700 hover:bg-blue-100" }),
                disabled: is_thinking(),
                onclick: { let db = db.clone(); let agent_system = agent_system.clone(); let work_item = work_item.clone(); let messages_resource = messages_resource.clone(); let work_items_resource = work_items_resource.clone(); let execution_runs_resource = execution_runs_resource.clone(); let artifacts_resource = artifacts_resource.clone();
                    move |_| { run_work_item_now(db.clone(), agent_system.clone(), work_item.clone(), messages_resource.clone(), visible_messages, selected_work_item, is_thinking, work_items_resource.clone(), execution_runs_resource.clone(), artifacts_resource.clone(), work_item_events_resource.clone(), selected_execution_run_id); } },
                MaterialIcon { name: if is_thinking() { "hourglass_top" } else if can_retry { "replay" } else { "smart_toy" }, size: 14 }
                if is_thinking() { "Running" } else if can_retry { "Retry" } else { "Run Now" } }
            button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                onclick: { let prompt = work_item.prompt.clone(); move |_| copy_to_clipboard(prompt.clone()) },
                MaterialIcon { name: "content_copy", size: 14 } "Copy Prompt" }
            button { class: "inline-flex items-center gap-2 rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-amber-800 hover:bg-amber-100 transition-all",
                onclick: { let db = db.clone(); let work_item = work_item.clone(); let work_items_resource = work_items_resource.clone();
                    move |_| { update_work_item_manual_status(db.clone(), work_item.clone(), "blocked".to_string(), selected_work_item, work_items_resource.clone(), work_item_events_resource.clone()); } },
                MaterialIcon { name: "pause_circle", size: 14 } "Mark Blocked" }
            button { class: "inline-flex items-center gap-2 rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-emerald-700 hover:bg-emerald-100 transition-all",
                onclick: { let db = db.clone(); let work_item = work_item.clone(); let work_items_resource = work_items_resource.clone();
                    move |_| { update_work_item_manual_status(db.clone(), work_item.clone(), "done".to_string(), selected_work_item, work_items_resource.clone(), work_item_events_resource.clone()); } },
                MaterialIcon { name: "task_alt", size: 14 } "Mark Done" }
        }
    }
}
