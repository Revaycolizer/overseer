// src/routes/conversations/layout.rs
use crate::backend;
use super::state::ConversationsState;
use dioxus::prelude::*;

#[component]
pub fn ConversationsHeader(
    state: ConversationsState,
    conversation_title: String,
    route_reason: Option<String>,
    route_label: Option<String>,
    personality_label: Option<String>,
    latest_run: Option<backend::ExecutionRun>,
    selected_run: Option<backend::ExecutionRun>,
    is_first_msg: bool,
) -> Element {
    let _ = &state;

    rsx! {
        if !is_first_msg {
        div { class: "border-b border-slate-200 bg-white/90 backdrop-blur-sm px-3 sm:px-6 py-3",
            div { class: "max-w-3xl mx-auto flex items-center justify-between gap-4",
                div { class: "min-w-0",
                    h1 { class: "text-base font-semibold text-slate-900 truncate", "{conversation_title}" }
                    if let Some(reason) = route_reason {
                        p { class: "text-xs text-slate-400 mt-0.5 truncate", "{reason}" }
                    }
                }
            }
            if let Some(run) = selected_run {
                if let Some(error_message) = run.error_message.filter(|v| !v.trim().is_empty()) {
                    div { class: "max-w-3xl mx-auto mt-4 rounded-2xl border border-red-200 bg-red-50 px-4 py-3 flex items-start gap-3",
                        div { class: "mt-0.5 size-6 rounded-full bg-red-100 flex items-center justify-center text-red-600 text-xs font-black", "!" }
                        div { class: "min-w-0",
                            p { class: "text-xs font-black uppercase tracking-[0.2em] text-red-600", "Latest Run Failed" }
                            p { class: "text-xs text-red-700 mt-1 whitespace-pre-wrap break-words", "{error_message}" }
                        }
                    }
                }
            }
        }
        }
    }
}
