use dioxus::prelude::*;

#[component]
pub fn IntelligenceModel(
    default_chat_model: String,
    title_model: String,
    available_models: Vec<String>,
    on_change: EventHandler<()>,
) -> Element {
    let state = crate::state::use_global_state();

    rsx! {
        div { class: "rounded-2xl bg-white/75 p-6 space-y-5",
            div {
                p { class: "text-[11px] font-bold uppercase tracking-[0.2em] text-slate-400", "Model Selection" }
                h4 { class: "text-lg font-black text-slate-900 mt-2", "Choose runtime and utility models" }
            }
            div { class: "space-y-4",
                div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                    div { class: "md:pr-4",
                        label { class: "text-base font-bold text-slate-900 block mb-1", "Default Runtime Model" }
                        p { class: "text-sm text-slate-500", "Fallback model for chat, planners, supervisors, and new records when a saved model is unavailable." }
                    }
                    select {
                        class: "bg-white border-slate-200 rounded-xl text-sm w-full md:w-72 focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 cursor-pointer font-medium text-slate-700 transition-all",
                        value: "{default_chat_model}",
                        onchange: move |evt| {
                            let db = state.read().db.clone();
                            let on_change = on_change.clone();
                            spawn(async move {
                                let _ = crate::backend::set_setting(&db, "default_chat_model", &evt.value()).await;
                                on_change.call(());
                            });
                        },
                        for model in &available_models {
                            option { value: "{model}", selected: *model == default_chat_model, "{model}" }
                        }
                    }
                }

                div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                    div { class: "md:pr-4",
                        label { class: "text-base font-bold text-slate-900 block mb-1", "Utility Model" }
                        p { class: "text-sm text-slate-500", "Lightweight model for titles and summaries." }
                    }
                    select {
                        class: "bg-white border-slate-200 rounded-xl text-sm w-full md:w-72 focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 cursor-pointer font-medium text-slate-700 transition-all",
                        value: "{title_model}",
                        onchange: move |evt| {
                            let db = state.read().db.clone();
                            let on_change = on_change.clone();
                            spawn(async move {
                                let _ = crate::backend::set_setting(&db, "title_generator_model", &evt.value()).await;
                                on_change.call(());
                            });
                        },
                        for model in &available_models {
                            option { value: "{model}", selected: *model == title_model, "{model}" }
                        }
                    }
                }
            }
        }
    }
}
