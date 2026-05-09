// src/routes/conversations/work_item_panel_extras.rs
use super::route_summary::prettify_token;
use super::work_item_handoff::create_work_item_handoff;
use super::work_item_ops::parse_work_item_runtime_metadata;
use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn WorkItemBlockedResolution(
    db: std::sync::Arc<crate::db::Database>,
    work_item: backend::WorkItem,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
) -> Element {
    rsx! {
        div { class: "rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 space-y-3",
            div {
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-amber-700", "Blocked Resolution" }
                p { class: "text-xs text-amber-900 mt-1 leading-relaxed", "This work item is blocked. Retry with the latest artifacts, hand it off, or seed the composer with an investigation prompt." }
            }
            div { class: "flex flex-wrap gap-2",
                button { class: "inline-flex items-center gap-2 rounded-xl border border-blue-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-blue-700 hover:bg-blue-50 transition-all",
                    onclick: { let work_item = work_item.clone();
                        move |_| {
                            starter_prompt.set(Some(format!("Investigate why this work item is blocked and propose the next best step.\n\nWork item: {}\nPrompt: {}\nLatest result: {}",
                                work_item.title, work_item.prompt,
                                parse_work_item_runtime_metadata(&work_item).latest_response_excerpt.unwrap_or_else(|| "No latest result summary available.".to_string())
                            )));
                            selected_work_item.set(Some(work_item.clone()));
                        }
                    },
                    MaterialIcon { name: "manage_search", size: 14 } "Investigate"
                }
                button { class: "inline-flex items-center gap-2 rounded-xl border border-violet-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-violet-700 hover:bg-violet-50 transition-all",
                    onclick: { let db = db.clone(); let work_item = work_item.clone(); let work_items_resource = work_items_resource.clone(); let work_item_events_resource = work_item_events_resource.clone();
                        move |_| { create_work_item_handoff(db.clone(), work_item.clone(), selected_work_item, work_items_resource.clone(), work_item_events_resource.clone()); }
                    },
                    MaterialIcon { name: "forward_to_inbox", size: 14 } "Create Handoff"
                }
            }
        }
    }
}

#[component]
pub fn WorkItemEventTimeline(work_item_events: Vec<backend::WorkItemEvent>) -> Element {
    rsx! {
        div { class: "rounded-xl border border-slate-200 bg-white px-4 py-3 space-y-3",
            div { class: "flex items-center justify-between gap-3",
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Activity Timeline" }
                span { class: "text-[10px] font-semibold text-slate-400", "{work_item_events.len()} event(s)" }
            }
            if work_item_events.is_empty() {
                p { class: "text-xs text-slate-500", "No work-item events yet. Run, retry, block, complete, or hand off this item to build a visible history." }
            } else {
                div { class: "space-y-2 max-h-64 overflow-y-auto custom-scrollbar pr-1",
                    for event in work_item_events.iter() {
                        div { class: "rounded-xl border border-slate-200 bg-slate-50 px-3 py-3 space-y-1",
                            div { class: "flex flex-wrap items-center justify-between gap-2",
                                div { class: "flex flex-wrap items-center gap-2",
                                    span { class: "text-[10px] font-black uppercase tracking-[0.16em] text-slate-500", "{prettify_token(&event.event_type)}" }
                                    if let Some(status) = event.status.clone().filter(|v| !v.trim().is_empty()) { span { class: "text-[10px] font-black uppercase tracking-[0.16em] text-slate-400", "{prettify_token(&status)}" } }
                                    if let Some(run_id) = event.execution_run_id { span { class: "text-[10px] font-semibold text-slate-400", "Run #{run_id}" } }
                                }
                                span { class: "text-[10px] font-semibold text-slate-400", "{event.created_at}" }
                            }
                            if let Some(summary) = event.summary.clone().filter(|v| !v.trim().is_empty()) { p { class: "text-xs text-slate-700 leading-relaxed whitespace-pre-wrap break-words", "{summary}" } }
                        }
                    }
                }
            }
        }
    }
}
