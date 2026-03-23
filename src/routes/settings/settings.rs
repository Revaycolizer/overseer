use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn Settings() -> Element {
    let state = crate::state::use_global_state();
    let db = state.read().db.clone();

    let mut settings = use_resource(move || {
        let db = db.clone();
        async move { crate::backend::get_all_settings(&db).await }
    });

    let mut models =
        use_resource(move || async move { crate::backend::get_available_models().await });

    let current_settings = settings.read().cloned().unwrap_or_default();
    let available_models = models.read().cloned().unwrap_or_default();
    let first_available_model = available_models
        .first()
        .cloned()
        .unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());

    let title_model = current_settings
        .get("title_generator_model")
        .cloned()
        .unwrap_or_else(|| first_available_model.clone());

    let default_chat_model = current_settings
        .get("default_chat_model")
        .cloned()
        .unwrap_or_else(|| first_available_model.clone());

    let max_iterations = current_settings
        .get("max_iterations")
        .cloned()
        .unwrap_or_else(|| "10".to_string());

    let tool_timeout_ms = current_settings
        .get("tool_timeout_ms")
        .cloned()
        .unwrap_or_else(|| "45000".to_string());

    let model_timeout_ms = current_settings
        .get("model_timeout_ms")
        .cloned()
        .unwrap_or_else(|| "90000".to_string());

    let runtime_dir = crate::engine::utils::discovery::runtime_dir()
        .display()
        .to_string();
    let python_command = crate::engine::utils::discovery::python_command_label();

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-slate-50 overflow-y-auto custom-scrollbar",
            main { class: "flex-1 p-6 md:p-12 max-w-4xl mx-auto w-full",
                // Header section
                div { class: "mb-12 border-b border-slate-200 pb-8",
                    h2 { class: "text-4xl font-extrabold tracking-tight text-slate-900 mb-3",
                        "Settings"
                    }
                    p { class: "text-lg text-slate-600 max-w-2xl",
                        "Tailor your AI experience. Manage local models, agent constraints, and application preferences."
                    }
                }

                div { class: "space-y-12",
                    // AI Configuration Section
                    section {
                        div { class: "flex items-center gap-3 mb-8",
                            div { class: "p-2 bg-blue-100 rounded-lg text-blue-600",
                                MaterialIcon { name: "psychology", size: 24 }
                            }
                            div {
                                h3 { class: "text-xl font-bold text-slate-900", "Intelligence Engine" }
                                p { class: "text-sm text-slate-500", "Configure models and execution limits." }
                            }
                        }

                        div { class: "grid gap-6",
                            div { class: "group flex flex-col md:flex-row md:items-center justify-between p-6 bg-white rounded-2xl border border-slate-200 shadow-sm hover:shadow-md transition-all duration-200",
                                div { class: "mb-4 md:mb-0",
                                    label { class: "text-base font-bold text-slate-900 block mb-1",
                                        "Default Runtime Model"
                                    }
                                    p { class: "text-sm text-slate-500",
                                        "Fallback model for chat, planners, supervisors, and new records when a saved model is unavailable."
                                    }
                                }
                                select {
                                    class: "bg-slate-50 border-slate-200 rounded-xl text-sm w-full md:w-72 focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 shadow-sm cursor-pointer font-medium text-slate-700 transition-all",
                                    value: "{default_chat_model}",
                                    onchange: move |evt| {
                                        let db = state.read().db.clone();
                                        let mut settings = settings.clone();
                                        spawn(async move {
                                            let _ = crate::backend::set_setting(&db, "default_chat_model", &evt.value()).await;
                                            settings.restart();
                                        });
                                    },
                                    for model in &available_models {
                                        option { value: "{model}", selected: *model == default_chat_model, "{model}" }
                                    }
                                }
                            }

                            // Max Iterations
                            div { class: "group flex flex-col md:flex-row md:items-center justify-between p-6 bg-white rounded-2xl border border-slate-200 shadow-sm hover:shadow-md transition-all duration-200",
                                div { class: "mb-4 md:mb-0",
                                    label { class: "text-base font-bold text-slate-900 block mb-1",
                                        "Maximum Tool Iterations"
                                    }
                                    p { class: "text-sm text-slate-500",
                                        "Limit how many times an agent can call tools per step."
                                    }
                                }
                                div { class: "flex items-center gap-3 w-full md:w-72",
                                    input {
                                        r#type: "number",
                                        class: "bg-slate-50 border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 shadow-sm font-medium text-slate-700",
                                        value: "{max_iterations}",
                                        min: "1",
                                        max: "50",
                                        onchange: move |evt| {
                                            let db = state.read().db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::set_setting(&db, "max_iterations", &evt.value()).await;
                                                settings.restart();
                                            });
                                        }
                                    }
                                    span { class: "text-xs font-bold text-slate-400 uppercase tracking-widest", "Turns" }
                                }
                            }

                            div { class: "group flex flex-col md:flex-row md:items-center justify-between p-6 bg-white rounded-2xl border border-slate-200 shadow-sm hover:shadow-md transition-all duration-200",
                                div { class: "mb-4 md:mb-0",
                                    label { class: "text-base font-bold text-slate-900 block mb-1",
                                        "Tool Timeout"
                                    }
                                    p { class: "text-sm text-slate-500",
                                        "Maximum runtime allowed for each tool invocation."
                                    }
                                }
                                div { class: "flex items-center gap-3 w-full md:w-72",
                                    input {
                                        r#type: "number",
                                        class: "bg-slate-50 border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 shadow-sm font-medium text-slate-700",
                                        value: "{tool_timeout_ms}",
                                        min: "1000",
                                        max: "300000",
                                        step: "1000",
                                        onchange: move |evt| {
                                            let db = state.read().db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::set_setting(&db, "tool_timeout_ms", &evt.value()).await;
                                                settings.restart();
                                            });
                                        }
                                    }
                                    span { class: "text-xs font-bold text-slate-400 uppercase tracking-widest", "ms" }
                                }
                            }

                            div { class: "group flex flex-col md:flex-row md:items-center justify-between p-6 bg-white rounded-2xl border border-slate-200 shadow-sm hover:shadow-md transition-all duration-200",
                                div { class: "mb-4 md:mb-0",
                                    label { class: "text-base font-bold text-slate-900 block mb-1",
                                        "Model Timeout"
                                    }
                                    p { class: "text-sm text-slate-500",
                                        "Maximum time allowed for planner, supervisor, and synthesis model calls."
                                    }
                                }
                                div { class: "flex items-center gap-3 w-full md:w-72",
                                    input {
                                        r#type: "number",
                                        class: "bg-slate-50 border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 shadow-sm font-medium text-slate-700",
                                        value: "{model_timeout_ms}",
                                        min: "1000",
                                        max: "600000",
                                        step: "1000",
                                        onchange: move |evt| {
                                            let db = state.read().db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::set_setting(&db, "model_timeout_ms", &evt.value()).await;
                                                settings.restart();
                                            });
                                        }
                                    }
                                    span { class: "text-xs font-bold text-slate-400 uppercase tracking-widest", "ms" }
                                }
                            }

                            // Title Model
                            div { class: "group flex flex-col md:flex-row md:items-center justify-between p-6 bg-white rounded-2xl border border-slate-200 shadow-sm hover:shadow-md transition-all duration-200 opacity-80",
                                div { class: "mb-4 md:mb-0",
                                    label { class: "text-base font-bold text-slate-900 block mb-1",
                                        "Utility Model"
                                    }
                                    p { class: "text-sm text-slate-500",
                                        "Lightweight model for titles and summaries."
                                    }
                                }
                                select {
                                    class: "bg-slate-50 border-slate-200 rounded-xl text-sm w-full md:w-72 focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 shadow-sm cursor-pointer font-medium text-slate-700 transition-all",
                                    value: "{title_model}",
                                    onchange: move |evt| {
                                        let db = state.read().db.clone();
                                        let mut settings = settings.clone();
                                        spawn(async move {
                                            let _ = crate::backend::set_setting(&db, "title_generator_model", &evt.value()).await;
                                            settings.restart();
                                        });
                                    },
                                    for model in &available_models {
                                        option { value: "{model}", selected: *model == title_model, "{model}" }
                                    }
                                }
                            }
                        }
                    }

                    // System & Security Section
                    section {
                        div { class: "flex items-center gap-3 mb-8",
                            div { class: "p-2 bg-slate-200 rounded-lg text-slate-600",
                                MaterialIcon { name: "shield", size: 24 }
                            }
                            div {
                                h3 { class: "text-xl font-bold text-slate-900", "System Integrity" }
                                p { class: "text-sm text-slate-500", "Privacy settings and local environment control." }
                            }
                        }

                        div { class: "grid gap-6",
                            div { class: "group flex items-center justify-between p-6 bg-white rounded-2xl border border-slate-200 shadow-sm hover:shadow-md transition-all duration-200",
                                div {
                                    p { class: "text-base font-bold text-slate-900 block mb-1",
                                        "Strict Offline Mode"
                                    }
                                    p { class: "text-sm text-slate-500",
                                        "Force all processing to happen on your local machine."
                                    }
                                }
                                div { class: "relative inline-flex items-center cursor-pointer",
                                    input {
                                        checked: true,
                                        class: "sr-only peer",
                                        r#type: "checkbox",
                                    }
                                    div { class: "w-14 h-8 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[4px] after:left-[4px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-6 after:w-6 after:transition-all peer-checked:bg-blue-600" }
                                }
                            }

                            div { class: "p-6 bg-white rounded-2xl border border-slate-200 shadow-sm",
                                div { class: "mb-4",
                                    p { class: "text-base font-bold text-slate-900 mb-1", "Runtime Diagnostics" }
                                    p { class: "text-sm text-slate-500", "Current local runtime paths used for tool staging and Python execution." }
                                }
                                div { class: "space-y-3 text-sm",
                                    div {
                                        span { class: "block text-xs font-bold text-slate-400 uppercase tracking-widest mb-1", "Python Command" }
                                        code { class: "block bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 text-slate-700 break-all", "{python_command}" }
                                    }
                                    div {
                                        span { class: "block text-xs font-bold text-slate-400 uppercase tracking-widest mb-1", "Runtime Directory" }
                                        code { class: "block bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 text-slate-700 break-all", "{runtime_dir}" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Footer Branding
                div { class: "mt-20 pt-8 border-t border-slate-200 text-center",
                    p { class: "text-sm font-bold text-slate-400 uppercase tracking-widest",
                        "Overseer v1.0.0 — Empowering Agents Locally"
                    }
                }
            }
        }
    }
}
