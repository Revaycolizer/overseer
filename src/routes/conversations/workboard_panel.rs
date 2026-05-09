// src/routes/conversations/workboard_panel.rs
use super::work_item_card::WorkItemCard;
use crate::backend;
use dioxus::prelude::*;

#[component]
pub fn WorkboardPanel(
    db: std::sync::Arc<crate::db::Database>,
    agent_system: std::sync::Arc<crate::engine::system::AgentSystem>,
    messages_resource: Resource<Vec<backend::Message>>,
    visible_messages: Signal<Vec<backend::Message>>,
    work_items: Vec<backend::WorkItem>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    is_thinking: Signal<bool>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    execution_runs_resource: Resource<Vec<backend::ExecutionRun>>,
    artifacts_resource: Resource<Vec<backend::Artifact>>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
    selected_execution_run_id: Signal<Option<i64>>,
) -> Element {
    rsx! {
        div { class: "border-l border-slate-200 border-b border-slate-200 bg-white/95 backdrop-blur-sm",
            div { class: "px-5 py-4 space-y-3",
                div {
                    p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Workboard" }
                    h3 { class: "text-sm font-black text-slate-900 mt-1", "Explicit work items for humans and agents to coordinate through." }
                }

                if work_items.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-slate-200 bg-slate-50 px-4 py-3 text-xs text-slate-500 leading-relaxed",
                        "No work items yet. Create one from an artifact surface to turn a prompt into a tracked unit of work."
                    }
                } else {
                    div { class: "space-y-2 max-h-72 overflow-y-auto custom-scrollbar pr-1",
                        for work_item in work_items {
                            WorkItemCard {
                                key: "{work_item.id}",
                                db: db.clone(),
                                agent_system: agent_system.clone(),
                                messages_resource: messages_resource.clone(),
                                visible_messages: visible_messages,
                                work_item: work_item,
                                selected_work_item: selected_work_item,
                                is_thinking: is_thinking,
                                starter_prompt: starter_prompt,
                                work_items_resource: work_items_resource.clone(),
                                execution_runs_resource: execution_runs_resource.clone(),
                                artifacts_resource: artifacts_resource.clone(),
                                work_item_events_resource: work_item_events_resource.clone(),
                                selected_execution_run_id: selected_execution_run_id,
                            }
                        }
                    }
                }
            }
        }
    }
}
