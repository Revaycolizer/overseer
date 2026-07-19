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
        div { class: "flex flex-wrap items-center justify-center gap-2",
            for (label, prompt) in prompts {
                button {
                    class: "rounded-full border border-slate-200 bg-white hover:bg-slate-50 hover:border-slate-300 transition-colors px-4 py-2 text-sm font-medium text-slate-600",
                    onclick: {
                        let prompt = prompt.to_string();
                        move |_| on_prompt.call(prompt.clone())
                    },
                    "{label}"
                }
            }
        }
    }
}
