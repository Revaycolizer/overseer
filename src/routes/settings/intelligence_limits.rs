use dioxus::prelude::*;

#[component]
pub fn IntelligenceLimits(
    max_iterations: String,
    tool_timeout_ms: String,
    model_timeout_ms: String,
    on_change: EventHandler<()>,
) -> Element {
    let state = crate::state::use_global_state();

    rsx! {
        div { class: "rounded-2xl bg-white/75 p-6 space-y-5",
            div {
                p { class: "text-[11px] font-bold uppercase tracking-[0.2em] text-slate-400", "Execution Limits" }
                h4 { class: "text-lg font-black text-slate-900 mt-2", "Control runtime boundaries" }
            }
            div { class: "space-y-4",
                div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                    div { class: "md:pr-4",
                        label { class: "text-base font-bold text-slate-900 block mb-1", "Maximum Tool Iterations" }
                        p { class: "text-sm text-slate-500", "Limit how many times an agent can call tools per step." }
                    }
                    div { class: "flex items-center gap-3 w-full md:w-72",
                        input {
                            r#type: "number",
                            class: "bg-white border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 font-medium text-slate-700",
                            value: "{max_iterations}",
                            min: "1",
                            max: "50",
                            onchange: move |evt| {
                                let db = state.read().db.clone();
                                let on_change = on_change.clone();
                                spawn(async move {
                                    let _ = crate::backend::set_setting(&db, "max_iterations", &evt.value()).await;
                                    on_change.call(());
                                });
                            }
                        }
                        span { class: "text-xs font-bold text-slate-400 uppercase tracking-widest", "Turns" }
                    }
                }

                div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                    div { class: "md:pr-4",
                        label { class: "text-base font-bold text-slate-900 block mb-1", "Tool Timeout" }
                        p { class: "text-sm text-slate-500", "Maximum runtime allowed for each tool invocation." }
                    }
                    div { class: "flex items-center gap-3 w-full md:w-72",
                        input {
                            r#type: "number",
                            class: "bg-white border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 font-medium text-slate-700",
                            value: "{tool_timeout_ms}",
                            min: "1000",
                            max: "300000",
                            step: "1000",
                            onchange: move |evt| {
                                let db = state.read().db.clone();
                                let on_change = on_change.clone();
                                spawn(async move {
                                    let _ = crate::backend::set_setting(&db, "tool_timeout_ms", &evt.value()).await;
                                    on_change.call(());
                                });
                            }
                        }
                        span { class: "text-xs font-bold text-slate-400 uppercase tracking-widest", "ms" }
                    }
                }

                div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                    div { class: "md:pr-4",
                        label { class: "text-base font-bold text-slate-900 block mb-1", "Model Timeout" }
                        p { class: "text-sm text-slate-500", "Maximum time allowed for planner, supervisor, and synthesis model calls." }
                    }
                    div { class: "flex items-center gap-3 w-full md:w-72",
                        input {
                            r#type: "number",
                            class: "bg-white border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 font-medium text-slate-700",
                            value: "{model_timeout_ms}",
                            min: "1000",
                            max: "600000",
                            step: "1000",
                            onchange: move |evt| {
                                let db = state.read().db.clone();
                                let on_change = on_change.clone();
                                spawn(async move {
                                    let _ = crate::backend::set_setting(&db, "model_timeout_ms", &evt.value()).await;
                                    on_change.call(());
                                });
                            }
                        }
                        span { class: "text-xs font-bold text-slate-400 uppercase tracking-widest", "ms" }
                    }
                }
            }
        }
    }
}
