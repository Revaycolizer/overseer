// src/routes/conversations/registry_panel.rs
use crate::backend;
use dioxus::prelude::*;

#[component]
pub fn RuntimeRegistryPanel(status: Option<backend::RuntimeRegistryStatus>) -> Element {
    let status = status.unwrap_or(backend::RuntimeRegistryStatus {
        status: "unknown".to_string(),
        refreshed_at: None,
        error: None,
    });

    let tone_class = match status.status.as_str() {
        "live" => "border-emerald-200 bg-emerald-50 text-emerald-700",
        "error" => "border-red-200 bg-red-50 text-red-700",
        _ => "border-slate-200 bg-slate-50 text-slate-600",
    };

    rsx! {
        div { class: "border-b border-slate-200 bg-white p-6 space-y-3",
            p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Runtime Registry" }
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-black text-slate-900", "Live runtime sync" }
                span { class: "rounded-full border px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] {tone_class}",
                    "{status.status}"
                }
            }
            p { class: "text-xs text-slate-500 leading-relaxed",
                "Shows whether the staged agent/tool/supervisor registry is currently live in the runtime."
            }
            if let Some(refreshed_at) = status.refreshed_at {
                p { class: "text-xs text-slate-600", "Last refreshed: {refreshed_at}" }
            } else {
                p { class: "text-xs text-slate-500", "No refresh timestamp recorded yet." }
            }
            if let Some(error) = status.error.filter(|value| !value.trim().is_empty()) {
                p { class: "text-xs text-red-700 whitespace-pre-wrap break-words", "{error}" }
            }
        }
    }
}
