use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

use crate::backend::{ExecutionEvent, ExecutionRun};

#[derive(Props, Clone, PartialEq)]
pub struct ExecutionTracePanelProps {
    pub heading: String,
    pub subtitle: String,
    pub empty_title: String,
    pub empty_body: String,
    pub runs: Vec<ExecutionRun>,
    pub events: Vec<ExecutionEvent>,
    pub selected_run_id: Signal<Option<i64>>,
    pub runs_loading: bool,
    pub events_loading: bool,
}

#[component]
pub fn ExecutionTracePanel(mut props: ExecutionTracePanelProps) -> Element {
    let selected_run = props
        .runs
        .iter()
        .find(|run| Some(run.id) == (props.selected_run_id)())
        .cloned();

    rsx! {
        div { class: "h-full min-h-0 bg-white border-l border-slate-200 overflow-hidden flex flex-col",
            div { class: "px-5 py-4 border-b border-slate-200 bg-slate-50/80",
                div { class: "flex items-center gap-2 mb-1",
                    MaterialIcon { name: "timeline", size: 18, color: MaterialIconColor::Custom("#2563eb".into()) }
                    h3 { class: "text-sm font-black uppercase tracking-widest text-slate-900", "{props.heading}" }
                }
                p { class: "text-xs text-slate-500 leading-relaxed", "{props.subtitle}" }
            }

            div { class: "flex-1 min-h-0 overflow-y-auto custom-scrollbar p-4 space-y-4",
                if props.runs_loading && props.runs.is_empty() {
                    TraceLoader {}
                } else if props.runs.is_empty() {
                    EmptyTraceState {
                        title: props.empty_title.clone(),
                        body: props.empty_body.clone(),
                    }
                } else {
                    div { class: "space-y-3",
                        div { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400 px-1",
                            "Recent Runs"
                        }
                        div { class: "space-y-2",
                            for run in props.runs.iter().cloned() {
                                button {
                                    key: "{run.id}",
                                    class: format!(
                                        "w-full text-left rounded-2xl border p-3 transition-all {}",
                                        if (props.selected_run_id)() == Some(run.id) {
                                            "border-blue-300 bg-blue-50/60 shadow-sm"
                                        } else {
                                            "border-slate-200 bg-white hover:border-slate-300"
                                        }
                                    ),
                                    onclick: {
                                        let run_id = run.id;
                                        move |_| props.selected_run_id.set(Some(run_id))
                                    },
                                    div { class: "flex items-start justify-between gap-3",
                                        div { class: "min-w-0",
                                            p { class: "text-xs font-bold text-slate-900 line-clamp-2", "{run.user_query}" }
                                            p { class: "text-[11px] text-slate-400 mt-1",
                                                {run.started_at.format("%Y-%m-%d %H:%M").to_string()}
                                            }
                                        }
                                        StatusBadge { status: run.status.clone() }
                                    }
                                }
                            }
                        }

                        if let Some(run) = selected_run {
                            div { class: "rounded-3xl border border-slate-200 bg-slate-50/70 p-4 space-y-4",
                                div { class: "flex items-center justify-between gap-3",
                                    div {
                                        p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Selected Run" }
                                        h4 { class: "text-sm font-black text-slate-900 mt-1", "Run #{run.id}" }
                                    }
                                    StatusBadge { status: run.status.clone() }
                                }
                                div { class: "grid grid-cols-1 gap-3",
                                    TraceMetaRow { label: "Started".to_string(), value: run.started_at.format("%Y-%m-%d %H:%M:%S").to_string() }
                                    TraceMetaRow {
                                        label: "Finished".to_string(),
                                        value: run.finished_at
                                            .map(|value| value.format("%Y-%m-%d %H:%M:%S").to_string())
                                            .unwrap_or_else(|| "In progress".to_string())
                                    }
                                }
                                if let Some(final_response) = run.final_response.clone().filter(|value| !value.trim().is_empty()) {
                                    TraceTextBlock {
                                        label: "Final Response".to_string(),
                                        value: final_response,
                                    }
                                } else if let Some(error_message) = run.error_message.clone().filter(|value| !value.trim().is_empty()) {
                                    TraceTextBlock {
                                        label: "Failure".to_string(),
                                        value: error_message,
                                    }
                                }
                            }

                            div { class: "space-y-3",
                                div { class: "flex items-center justify-between px-1",
                                    span { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Execution Timeline" }
                                    if props.events_loading {
                                        span { class: "text-[10px] font-medium text-slate-400", "Refreshing..." }
                                    }
                                }

                                if props.events_loading && props.events.is_empty() {
                                    TraceLoader {}
                                } else if props.events.is_empty() {
                                    div { class: "rounded-2xl border border-dashed border-slate-200 bg-white p-5 text-xs text-slate-400",
                                        "No execution events were captured for this run yet."
                                    }
                                } else {
                                    div { class: "space-y-3",
                                        for event in props.events.iter().cloned() {
                                            div {
                                                key: "{event.id}",
                                                class: "rounded-2xl border border-slate-200 bg-white p-4 space-y-2 shadow-sm",
                                                div { class: "flex items-start justify-between gap-3",
                                                    div {
                                                        p { class: "text-xs font-black text-slate-900 uppercase tracking-wide",
                                                            "{event.event_type.replace('_', \" \")}"
                                                        }
                                                        p { class: "text-[11px] text-slate-400 mt-1",
                                                            {format!("Step {} • {}", event.sequence, event.created_at.format("%H:%M:%S"))}
                                                        }
                                                    }
                                                    StatusBadge { status: event.status.clone() }
                                                }
                                                div { class: "flex flex-wrap gap-2",
                                                    if let Some(domain) = event.domain.clone().filter(|value| !value.trim().is_empty()) {
                                                        TraceChip { value: domain }
                                                    }
                                                    if let Some(tool_name) = event.tool_name.clone().filter(|value| !value.trim().is_empty()) {
                                                        TraceChip { value: format!("tool: {}", tool_name) }
                                                    }
                                                    if let Some(model) = event.model.clone().filter(|value| !value.trim().is_empty()) {
                                                        TraceChip { value: model }
                                                    }
                                                }
                                                if let Some(step_text) = event.step_text.clone().filter(|value| !value.trim().is_empty()) {
                                                    TraceTextBlock {
                                                        label: "Step".to_string(),
                                                        value: step_text,
                                                    }
                                                }
                                                if let Some(message) = event.message.clone().filter(|value| !value.trim().is_empty()) {
                                                    TraceTextBlock {
                                                        label: "Message".to_string(),
                                                        value: message,
                                                    }
                                                }
                                                if let Some(details) = event.details.clone().filter(|value| !value.trim().is_empty()) {
                                                    TraceTextBlock {
                                                        label: "Details".to_string(),
                                                        value: details,
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatusBadge(status: String) -> Element {
    let style = match status.as_str() {
        "completed" | "success" => "bg-emerald-50 text-emerald-600 border-emerald-200",
        "failed" | "error" => "bg-red-50 text-red-600 border-red-200",
        "running" | "in_progress" => "bg-blue-50 text-blue-600 border-blue-200",
        _ => "bg-slate-100 text-slate-500 border-slate-200",
    };

    rsx! {
        span { class: format!("text-[10px] font-black uppercase tracking-widest px-2 py-1 rounded-full border {}", style),
            "{status.replace('_', \" \")}"
        }
    }
}

#[component]
fn TraceChip(value: String) -> Element {
    rsx! {
        span { class: "text-[10px] font-semibold px-2 py-1 rounded-full bg-slate-100 text-slate-500 border border-slate-200",
            "{value}"
        }
    }
}

#[component]
fn TraceMetaRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-white px-3 py-2",
            p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "{label}" }
            p { class: "text-xs font-semibold text-slate-700 mt-1 break-all", "{value}" }
        }
    }
}

#[component]
fn TraceTextBlock(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-white p-3 space-y-2",
            p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "{label}" }
            pre { class: "text-xs text-slate-700 whitespace-pre-wrap break-words font-mono leading-relaxed overflow-x-auto",
                "{value}"
            }
        }
    }
}

#[component]
fn EmptyTraceState(title: String, body: String) -> Element {
    rsx! {
        div { class: "rounded-3xl border border-dashed border-slate-200 bg-white p-8 text-center",
            div { class: "size-14 rounded-2xl bg-slate-50 border border-slate-200 flex items-center justify-center mx-auto mb-4",
                MaterialIcon { name: "query_stats", size: 24, color: MaterialIconColor::Custom("#94a3b8".into()) }
            }
            h4 { class: "text-sm font-black text-slate-900", "{title}" }
            p { class: "text-xs text-slate-500 mt-2 leading-relaxed", "{body}" }
        }
    }
}

#[component]
fn TraceLoader() -> Element {
    rsx! {
        div { class: "rounded-3xl border border-slate-200 bg-white p-8 flex items-center justify-center gap-3 text-slate-400",
            div { class: "size-5 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" }
            span { class: "text-xs font-medium", "Loading trace data..." }
        }
    }
}
