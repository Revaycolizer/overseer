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
    let mut selected_filter = use_signal(|| "all".to_string());
    let mut expanded_event_ids = use_signal(Vec::<i64>::new);

    let selected_run = props
        .runs
        .iter()
        .find(|run| Some(run.id) == (props.selected_run_id)())
        .cloned();
    let filtered_events: Vec<ExecutionEvent> = props
        .events
        .iter()
        .filter(|event| matches_trace_filter(event, &selected_filter()))
        .cloned()
        .collect();
    let filter_options = trace_filter_options(&props.events);

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

                                div { class: "flex flex-wrap gap-2",
                                    for (filter_key, filter_label, filter_count) in filter_options {
                                        button {
                                            key: "{filter_key}",
                                            class: format!(
                                                "rounded-full border px-3 py-1 text-[10px] font-black uppercase tracking-[0.18em] transition-all {}",
                                                if selected_filter() == filter_key {
                                                    "border-blue-200 bg-blue-50 text-blue-700"
                                                } else {
                                                    "border-slate-200 bg-white text-slate-500 hover:border-slate-300"
                                                }
                                            ),
                                            onclick: {
                                                let filter_key = filter_key.clone();
                                                move |_| selected_filter.set(filter_key.clone())
                                            },
                                            "{filter_label} ({filter_count})"
                                        }
                                    }
                                }

                                if props.events_loading && props.events.is_empty() {
                                    TraceLoader {}
                                } else if filtered_events.is_empty() {
                                    div { class: "rounded-2xl border border-dashed border-slate-200 bg-white p-5 text-xs text-slate-400",
                                        "No execution events match the current filter."
                                    }
                                } else {
                                    div { class: "space-y-3",
                                        for event in filtered_events {
                                            {
                                                let event_label = describe_event_type(&event.event_type);
                                                let event_category = classify_event(&event);
                                                let detail_label = detail_label_for_event(&event);
                                                let is_expanded = expanded_event_ids().contains(&event.id);

                                                rsx! {
                                                    div {
                                                        key: "{event.id}",
                                                        class: "rounded-2xl border border-slate-200 bg-white p-4 space-y-3 shadow-sm",
                                                        div { class: "flex items-start justify-between gap-3",
                                                            div {
                                                                p { class: "text-xs font-black text-slate-900 uppercase tracking-wide",
                                                                    "{event_label}"
                                                                }
                                                                p { class: "text-[11px] text-slate-400 mt-1",
                                                                    {format!("Step {} • {}", event.sequence, event.created_at.format("%H:%M:%S"))}
                                                                }
                                                            }
                                                            div { class: "flex items-center gap-2 shrink-0",
                                                                TraceChip { value: event_category }
                                                                StatusBadge { status: event.status.clone() }
                                                            }
                                                        }

                                                        div { class: "flex flex-wrap gap-2",
                                                            if let Some(domain) = event.domain.clone().filter(|value| !value.trim().is_empty()) {
                                                                TraceChip { value: prettify_token(&domain) }
                                                            }
                                                            if let Some(tool_name) = event.tool_name.clone().filter(|value| !value.trim().is_empty()) {
                                                                TraceChip { value: format!("Tool: {}", tool_name) }
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
                                                            div { class: "space-y-2",
                                                                button {
                                                                    class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500 hover:border-slate-300 hover:bg-white",
                                                                    onclick: {
                                                                        let event_id = event.id;
                                                                        move |_| {
                                                                            expanded_event_ids.with_mut(|ids| {
                                                                                if let Some(index) = ids.iter().position(|id| *id == event_id) {
                                                                                    ids.remove(index);
                                                                                } else {
                                                                                    ids.push(event_id);
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    MaterialIcon {
                                                                        name: if is_expanded { "expand_less" } else { "expand_more" },
                                                                        size: 16,
                                                                        color: MaterialIconColor::Custom("#64748b".into())
                                                                    }
                                                                    if is_expanded {
                                                                        "Hide Raw Details"
                                                                    } else {
                                                                        "Show Raw Details"
                                                                    }
                                                                }
                                                                if is_expanded {
                                                                    TraceTextBlock {
                                                                        label: detail_label,
                                                                        value: pretty_trace_details(&details),
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
            }
        }
    }
}

fn trace_filter_options(events: &[ExecutionEvent]) -> Vec<(String, String, usize)> {
    let filters = [
        ("all", "All"),
        ("decision", "Decision"),
        ("planning", "Planning"),
        ("tool", "Tools"),
        ("synthesis", "Synthesis"),
        ("failure", "Failure"),
    ];

    filters
        .into_iter()
        .map(|(key, label)| {
            let count = if key == "all" {
                events.len()
            } else {
                events
                    .iter()
                    .filter(|event| matches_trace_filter(event, key))
                    .count()
            };
            (key.to_string(), label.to_string(), count)
        })
        .collect()
}

fn matches_trace_filter(event: &ExecutionEvent, filter: &str) -> bool {
    match filter {
        "all" => true,
        "decision" => event.event_type == "orchestration_decision",
        "planning" => matches!(
            event.event_type.as_str(),
            "macro_plan" | "single_domain_route" | "micro_step" | "planning"
        ),
        "tool" => {
            event.tool_name.is_some()
                || matches!(
                    event.event_type.as_str(),
                    "tool_call" | "tool_execution" | "tool_result"
                )
        }
        "synthesis" => matches!(
            event.event_type.as_str(),
            "direct_chat" | "final_response" | "synthesis"
        ),
        "failure" => matches!(event.status.as_str(), "failed" | "error"),
        _ => true,
    }
}

fn classify_event(event: &ExecutionEvent) -> String {
    if matches!(event.status.as_str(), "failed" | "error") {
        return "Failure".to_string();
    }

    match event.event_type.as_str() {
        "orchestration_decision" => "Decision".to_string(),
        "macro_plan" | "single_domain_route" | "micro_step" | "planning" => "Planning".to_string(),
        "tool_call" | "tool_execution" | "tool_result" => "Tool".to_string(),
        "direct_chat" | "final_response" | "synthesis" => "Synthesis".to_string(),
        "approval_requested" | "approval_approved" | "approval_rejected" | "approval_cancelled" => {
            "Approval".to_string()
        }
        "action_requested" | "policy_allowed" | "policy_denied" | "action_started"
        | "action_completed" | "action_failed" | "action_timed_out" | "emergency_stop" => {
            "Kernel".to_string()
        }
        _ if event.tool_name.is_some() => "Tool".to_string(),
        _ => "System".to_string(),
    }
}

fn describe_event_type(event_type: &str) -> String {
    match event_type {
        "orchestration_decision" => "Orchestration Decision".to_string(),
        "single_domain_route" => "Single-Domain Route".to_string(),
        "macro_plan" => "Macro Plan".to_string(),
        "micro_step" => "Micro Step".to_string(),
        "tool_call" => "Tool Call".to_string(),
        "tool_execution" => "Tool Execution".to_string(),
        "tool_result" => "Tool Result".to_string(),
        "direct_chat" => "Direct Chat Response".to_string(),
        "final_response" => "Final Response".to_string(),
        "synthesis" => "Synthesis".to_string(),
        // Overseer Kernel v0.1 action-lifecycle events.
        "action_requested" => "Action Requested".to_string(),
        "policy_allowed" => "Policy · Allowed".to_string(),
        "policy_denied" => "Policy · Denied".to_string(),
        "approval_requested" => "Approval Requested".to_string(),
        "approval_approved" => "Approval · Approved".to_string(),
        "approval_rejected" => "Approval · Rejected".to_string(),
        "approval_cancelled" => "Approval · Cancelled".to_string(),
        "action_started" => "Action Started".to_string(),
        "action_completed" => "Action Completed".to_string(),
        "action_failed" => "Action Failed".to_string(),
        "action_timed_out" => "Action Timed Out".to_string(),
        "emergency_stop" => "Emergency Stop".to_string(),
        other => prettify_token(other),
    }
}

fn pretty_trace_details(details: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(details) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| details.to_string()),
        Err(_) => details.to_string(),
    }
}

fn detail_label_for_event(event: &ExecutionEvent) -> String {
    if event.event_type == "orchestration_decision" {
        "Decision Payload".to_string()
    } else if event.tool_name.is_some() {
        "Tool Payload".to_string()
    } else {
        "Event Details".to_string()
    }
}

fn prettify_token(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
            "{prettify_token(&status)}"
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
