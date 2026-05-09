// src/routes/conversations/empty_state.rs
use dioxus::prelude::*;

#[component]
pub fn EmptyConversationState(on_prompt: EventHandler<String>) -> Element {
    let prompts = [
        (
            "Research something",
            "Find the top three AI agent reliability practices and summarize them with evidence.",
        ),
        (
            "Plan a workflow",
            "Create a workflow plan for a daily website monitoring task with traceability.",
        ),
        (
            "Use a tool",
            "Generate a simple bar chart from the values 4, 8, and 12 with labels A, B, and C.",
        ),
    ];

    rsx! {
        div { class: "rounded-3xl border border-slate-200 bg-white shadow-sm p-8 space-y-6",
            div { class: "space-y-3",
                p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Mission Control" }
                h2 { class: "text-2xl font-black text-slate-900", "Start a conversation and let Overseer route the work." }
                p { class: "text-sm text-slate-500 leading-relaxed max-w-2xl",
                    "Chat is the front door. The orchestrator decides whether your request should stay conversational, go to a single domain, or expand into a multi-step execution run with traceability."
                }
            }
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-3",
                for (label, prompt) in prompts {
                    button {
                        class: "text-left rounded-2xl border border-slate-200 bg-slate-50 hover:bg-white hover:border-blue-200 transition-all p-4 space-y-2",
                        onclick: {
                            let prompt = prompt.to_string();
                            move |_| on_prompt.call(prompt.clone())
                        },
                        p { class: "text-xs font-black uppercase tracking-[0.18em] text-slate-500", "{label}" }
                        p { class: "text-sm text-slate-700 leading-relaxed", "{prompt}" }
                    }
                }
            }
        }
    }
}
