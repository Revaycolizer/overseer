use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

/// Inline test runner panel.
/// `supervisor_id`  — used to label the test run.
/// `local_code`     — read-only snapshot for the test call.
/// `on_close`       — called when the panel's close button is pressed.
///                    Pass `None` to hide the close button (e.g. in the
///                    fullscreen drawer where visibility is controlled by
///                    the parent).
#[component]
pub fn TestPanel(
    supervisor_id: i64,
    local_code: Signal<String>,
    on_close: Option<EventHandler<()>>,
) -> Element {
    let mut test_query = use_signal(|| String::new());
    let mut test_result = use_signal(|| String::new());
    let mut test_running = use_signal(|| false);

    let run_test = move |_| {
        let query = test_query();
        if query.trim().is_empty() {
            return;
        }
        let code = local_code();
        let sup_id = supervisor_id;
        test_running.set(true);
        test_result.set(String::new());
        spawn(async move {
            // TODO: replace with your actual backend call:
            // let result = crate::backend::test_supervisor(sup_id, &query, &code).await;
            let placeholder = format!(
                "// Supervisor #{sup_id} test run\n\
                 // Query  : {query}\n\
                 // Code   : {} bytes\n\
                 // Status : ✅ Syntax OK\n\
                 // Runtime: Dispatching to agent pool...\n\n\
                 // → Wire crate::backend::test_supervisor() to see live output.",
                code.len()
            );
            test_result.set(placeholder);
            test_running.set(false);
        });
    };

    rsx! {
        div { class: "flex flex-col h-full",

            // ── Header ─────────────────────────────────────────────────
            div { class: "flex items-center justify-between px-5 py-3 bg-slate-50 border-b border-[#E2E8F0] shrink-0",
                div { class: "flex items-center gap-2",
                    MaterialIcon {
                        name: "science",
                        size: 16,
                        color: MaterialIconColor::Custom("#3b82f6".into())
                    }
                    span {
                        class: "text-[10px] font-bold text-slate-500 uppercase tracking-widest",
                        "Test Runner"
                    }
                }
                if let Some(handler) = on_close {
                    button {
                        class: "p-1.5 rounded-md hover:bg-slate-100 text-slate-400 hover:text-slate-600 transition-colors",
                        onclick: move |_| handler.call(()),
                        MaterialIcon { name: "close", size: 16 }
                    }
                }
            }

            // ── Body ───────────────────────────────────────────────────
            div { class: "flex-1 flex flex-col p-5 gap-4 overflow-y-auto",

                // Query input
                div { class: "space-y-2",
                    label {
                        class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                        "Input Query"
                    }
                    textarea {
                        class: "w-full min-h-[88px] bg-[#F8FAFC] border border-[#E2E8F0] rounded-xl p-4 text-[#0F172A] font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6]/50 transition-all placeholder-slate-300",
                        placeholder: "Enter a test query — e.g. \"Summarise today's market news\"",
                        value: "{test_query}",
                        rows: 3,
                        oninput: move |e| test_query.set(e.value()),
                    }
                }

                // Run button
                button {
                    class: format!(
                        "w-full py-2.5 rounded-xl text-sm font-bold transition-all flex items-center justify-center gap-2 shadow-sm {}",
                        if test_running() {
                            "bg-slate-200 text-slate-400 cursor-not-allowed"
                        } else {
                            "bg-[#3B82F6] hover:bg-blue-500 text-white active:scale-95"
                        }
                    ),
                    disabled: test_running(),
                    onclick: run_test,
                    if test_running() {
                        span { class: "inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" }
                        "Running..."
                    } else {
                        MaterialIcon { name: "play_arrow", size: 18, color: MaterialIconColor::Light }
                        "Run Test"
                    }
                }

                // Output
                if !test_result().is_empty() {
                    div { class: "space-y-2",
                        label {
                            class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                            "Output"
                        }
                        pre {
                            class: "bg-[#0F172A] rounded-xl p-5 text-[#86efac] font-mono text-xs overflow-auto whitespace-pre-wrap leading-relaxed border border-[#1e293b]",
                            "{test_result}"
                        }
                    }
                }
            }
        }
    }
}
