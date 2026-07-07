use dioxus::prelude::*;
use crate::routes::settings::components::{ChecklistItem, SummaryTile};

#[component]
pub fn HealthSummary(
    health: Option<crate::backend::RuntimeHealth>,
    default_chat_model: String,
    title_model: String,
) -> Element {
    let health_ref = health.as_ref();
    let runtime_ready_count = health_ref
        .map(|h| {
            [
                h.ollama_reachable,
                h.runtime_model_available,
                h.protoc_path.is_some(),
                h.title_model_available,
                h.native_runtime,
            ]
            .into_iter()
            .filter(|item| *item)
            .count()
        })
        .unwrap_or(0);

    let summary = if let Some(h) = health_ref {
        if h.ollama_reachable && h.runtime_model_available && h.protoc_path.is_some() && h.native_runtime {
            if h.python_command.is_some() {
                "Local runtime is ready for chat, native tools, and optional Python extensions.".to_string()
            } else {
                "Local runtime is ready for chat and native tools. Python is optional for advanced extensions.".to_string()
            }
        } else if !h.ollama_reachable {
            "Ollama is offline. Start it before expecting chat or orchestration to run.".to_string()
        } else if !h.runtime_model_available {
            "Your selected runtime model is not installed yet. Pick an installed model or pull the missing one.".to_string()
        } else if h.protoc_path.is_none() {
            "protoc is missing. Build and generation flows will stay blocked until it is available.".to_string()
        } else if !h.native_runtime {
            "The native tool runtime is not ready yet. Restart Overseer and refresh this panel.".to_string()
        } else {
            "Runtime health is partially ready. Review the checks below before running deeper workflows.".to_string()
        }
    } else {
        "Checking local runtime health...".to_string()
    };

    let next_step = if let Some(h) = health_ref {
        if !h.ollama_reachable { "Start Ollama".to_string() }
        else if !h.runtime_model_available { "Choose installed model".to_string() }
        else if h.protoc_path.is_none() { "Install protoc".to_string() }
        else if !h.native_runtime { "Restart native runtime".to_string() }
        else if !h.title_model_available { "Pick utility model".to_string() }
        else { "Runtime looks healthy".to_string() }
    } else {
        "Checking runtime".to_string()
    };

    rsx! {
        section {
            div { class: "grid gap-4 xl:grid-cols-[minmax(0,1fr)_300px]",
                div { class: "rounded-3xl bg-white/80 p-6 md:p-8 space-y-6",
                    div { class: "flex items-start justify-between gap-4 flex-wrap",
                        div { class: "space-y-2 max-w-2xl",
                            p { class: "text-[11px] font-bold uppercase tracking-[0.24em] text-slate-400", "Runtime Command Center" }
                            h3 { class: "text-2xl font-black tracking-tight text-slate-900", "See if Overseer is actually ready before you run anything." }
                            p { class: "text-sm leading-relaxed text-slate-500", "{summary}" }
                        }
                        div { class: "rounded-2xl bg-slate-50/80 px-4 py-3 text-right min-w-[120px]",
                            p { class: "text-[10px] font-bold uppercase tracking-widest text-slate-400", "Checks Passing" }
                            p { class: "text-3xl font-black text-slate-900 leading-none mt-2", "{runtime_ready_count}/5" }
                        }
                    }
                    div { class: "grid gap-4 sm:grid-cols-3",
                        SummaryTile { title: "Runtime Model", value: default_chat_model, hint: "Primary model for chat, planners, and supervisors", icon: "smart_toy" }
                        SummaryTile { title: "Utility Model", value: title_model, hint: "Used for lighter naming and summary tasks", icon: "auto_awesome" }
                        SummaryTile { title: "Next Step", value: next_step, hint: "Most important fix before relying on the system", icon: "flag" }
                    }
                }
                div { class: "rounded-3xl bg-white/75 p-6 space-y-4",
                    div {
                        p { class: "text-[11px] font-bold uppercase tracking-[0.24em] text-slate-400", "First-Run Checklist" }
                        h3 { class: "text-lg font-black text-slate-900 mt-2", "Before you trust a run" }
                    }
                    if let Some(h) = health_ref {
                        ChecklistItem { label: "Ollama reachable", done: h.ollama_reachable, help: "The local model endpoint must answer before chat can work." }
                        ChecklistItem { label: "Runtime model installed", done: h.runtime_model_available, help: "Your selected default model should exist in Ollama." }
                        ChecklistItem { label: "Native tools active", done: h.native_runtime, help: "Core web, HTTP, and monitoring tools run natively now." }
                        ChecklistItem { label: "protoc detected", done: h.protoc_path.is_some(), help: "Build generation and protobuf-dependent flows use this binary." }
                        ChecklistItem { label: "Utility model ready", done: h.title_model_available, help: "Use a lighter model for titles and summaries." }
                    } else {
                        p { class: "text-sm text-slate-500", "Waiting for runtime checks to finish..." }
                    }
                }
            }
        }
    }
}
