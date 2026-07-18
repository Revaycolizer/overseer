use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

pub fn workflow_status_badge_class(status: &str) -> &'static str {
    match status { "running" => "bg-blue-100 text-blue-700 border-blue-200", "completed" => "bg-green-100 text-green-700 border-green-200", "failed" => "bg-red-100 text-red-700 border-red-200", "cancelled" | "canceled" => "bg-slate-100 text-slate-600 border-slate-300", _ => "bg-amber-100 text-amber-700 border-amber-200" }
}

pub fn workflow_status_tone(status: &str) -> &'static str {
    match status { "running" => "blue", "completed" => "green", "failed" => "red", _ => "amber" }
}

pub fn prettify_workflow_token(value: &str) -> String {
    value.replace('_', " ").split_whitespace().map(|w| { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }).collect::<Vec<_>>().join(" ")
}

#[component]
pub fn WorkflowMetricCard(label: String, value: String, tone: String) -> Element {
    let color = match tone.as_str() { "green" => "text-green-600", "red" => "text-red-600", "blue" => "text-blue-600", _ => "text-slate-800" };
    rsx! { div { class: "bg-white border border-slate-200 rounded-xl p-4 text-center shadow-sm",
        p { class: "text-xs font-bold text-slate-500 uppercase tracking-wider", "{label}" }
        p { class: "text-2xl font-black mt-1 {color}", "{value}" }
    } }
}

#[component]
pub fn WorkflowRunMetaRow(label: String, value: String) -> Element {
    rsx! { div { class: "flex items-center justify-between py-2 border-b border-slate-100 last:border-b-0",
        span { class: "text-xs font-semibold text-slate-500", "{label}" }
        span { class: "text-xs font-mono text-slate-800 font-medium", "{value}" }
    } }
}

#[component]
pub fn WorkflowRunSummaryBlock(label: String, value: String) -> Element {
    rsx! { div { class: "bg-gradient-to-br from-slate-50 to-white border border-slate-200 rounded-xl p-4 shadow-sm",
        p { class: "text-[10px] font-bold uppercase tracking-widest text-slate-400 mb-1", "{label}" }
        p { class: "text-sm font-bold text-slate-800", "{value}" }
    } }
}
