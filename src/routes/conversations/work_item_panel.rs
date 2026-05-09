// src/routes/conversations/work_item_panel.rs
use super::work_item_panel_parts::{WorkItemPanelHeader, WorkItemPanelActions};
use super::work_item_panel_extras::{WorkItemBlockedResolution, WorkItemEventTimeline};
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
pub fn WorkItemWorkspacePanel(
    db: std::sync::Arc<crate::db::Database>,
    agent_system: std::sync::Arc<crate::engine::system::AgentSystem>,
    work_item: backend::WorkItem,
    messages_resource: Resource<Vec<backend::Message>>,
    visible_messages: Signal<Vec<backend::Message>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    is_thinking: Signal<bool>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    execution_runs_resource: Resource<Vec<backend::ExecutionRun>>,
    artifacts_resource: Resource<Vec<backend::Artifact>>,
    work_item_events: Vec<backend::WorkItemEvent>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
    selected_execution_run_id: Signal<Option<i64>>,
) -> Element {
    let runtime_metadata = parse_work_item_runtime_metadata(&work_item);
    let latest_artifact_count = runtime_metadata.latest_artifact_ids.as_ref().map(|a| a.len()).unwrap_or(0);
    let can_retry = matches!(work_item.status.as_str(), "blocked" | "done" | "queued");

    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 to-white px-4 py-4 shadow-sm space-y-4",
            WorkItemPanelHeader { work_item: work_item.clone(), runtime_metadata_excerpt: runtime_metadata.latest_response_excerpt.clone(), runtime_metadata_status: runtime_metadata.latest_run_status.clone(), runtime_metadata_run_id: runtime_metadata.latest_run_id, runtime_metadata_executed_at: runtime_metadata.last_executed_at.clone(), latest_artifact_count: latest_artifact_count }
            WorkItemPanelActions { db: db.clone(), agent_system: agent_system, work_item: work_item.clone(), messages_resource: messages_resource, visible_messages: visible_messages, selected_work_item: selected_work_item, is_thinking: is_thinking, starter_prompt: starter_prompt, can_retry: can_retry, work_items_resource: work_items_resource.clone(), execution_runs_resource: execution_runs_resource, artifacts_resource: artifacts_resource, work_item_events_resource: work_item_events_resource.clone(), selected_execution_run_id: selected_execution_run_id }
            if work_item.status == "blocked" {
                WorkItemBlockedResolution { db: db, work_item: work_item.clone(), selected_work_item: selected_work_item, starter_prompt: starter_prompt, work_items_resource: work_items_resource, work_item_events_resource: work_item_events_resource.clone() }
            }
            WorkItemEventTimeline { work_item_events: work_item_events }
        }
    }
}
