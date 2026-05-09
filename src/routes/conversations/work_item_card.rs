// src/routes/conversations/work_item_card.rs
use super::work_item_card_parts::{WorkItemCardHeader, WorkItemCardActions};
use super::route_summary::prettify_token;
use super::work_item_ops::parse_work_item_runtime_metadata;
use super::work_item_runner::{run_work_item_now, update_work_item_manual_status};
use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn WorkItemCard(
    db: std::sync::Arc<crate::db::Database>,
    agent_system: std::sync::Arc<crate::engine::system::AgentSystem>,
    messages_resource: Resource<Vec<backend::Message>>,
    visible_messages: Signal<Vec<backend::Message>>,
    work_item: backend::WorkItem,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    is_thinking: Signal<bool>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    execution_runs_resource: Resource<Vec<backend::ExecutionRun>>,
    artifacts_resource: Resource<Vec<backend::Artifact>>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
    selected_execution_run_id: Signal<Option<i64>>,
) -> Element {
    let is_selected = selected_work_item().as_ref().map(|s| s.id == work_item.id).unwrap_or(false);
    let runtime_metadata = parse_work_item_runtime_metadata(&work_item);
    let latest_artifact_count = runtime_metadata.latest_artifact_ids.as_ref().map(|a| a.len()).unwrap_or(0);

    rsx! {
        div { class: format!("rounded-2xl border px-4 py-3 space-y-3 transition-all {}", if is_selected { "border-violet-200 bg-violet-50/60 shadow-sm" } else { "border-slate-200 bg-slate-50/80" }),
            WorkItemCardHeader { work_item: work_item.clone(), is_selected: is_selected, selected_work_item: selected_work_item, runtime_metadata_run_id: runtime_metadata.latest_run_id, runtime_metadata_status: runtime_metadata.latest_run_status.clone(), runtime_metadata_excerpt: runtime_metadata.latest_response_excerpt.clone(), runtime_metadata_executed_at: runtime_metadata.last_executed_at.clone(), latest_artifact_count: latest_artifact_count }
            WorkItemCardActions { db: db, agent_system: agent_system, work_item: work_item.clone(), messages_resource: messages_resource, visible_messages: visible_messages, selected_work_item: selected_work_item, is_thinking: is_thinking, starter_prompt: starter_prompt, work_items_resource: work_items_resource, execution_runs_resource: execution_runs_resource, artifacts_resource: artifacts_resource, work_item_events_resource: work_item_events_resource, selected_execution_run_id: selected_execution_run_id }
        }
    }
}

#[component]
pub fn WorkItemStatusBadge(status: String) -> Element {
    let class = match status.as_str() {
        "done" | "completed" => "bg-emerald-50 text-emerald-700 border-emerald-200",
        "in_progress" | "running" => "bg-blue-50 text-blue-700 border-blue-200",
        _ => "bg-amber-50 text-amber-700 border-amber-200",
    };
    rsx! {
        span { class: format!("inline-flex items-center rounded-full border px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] {}", class),
            "{prettify_token(&status)}"
        }
    }
}
