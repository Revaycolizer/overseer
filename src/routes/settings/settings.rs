use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn Settings() -> Element {
    let state = crate::state::use_global_state();
    let db = state.read().db.clone();
    let settings_db = db.clone();
    let health_db = db.clone();

    let mut settings = use_resource(move || {
        let db = settings_db.clone();
        async move { crate::backend::get_all_settings(&db).await }
    });

    let mut models =
        use_resource(move || async move { crate::backend::get_available_models().await });
    let mut runtime_health = use_resource(move || {
        let db = health_db.clone();
        async move { crate::backend::get_runtime_health(&db).await }
    });

    let current_settings = settings.read().cloned().unwrap_or_default();
    let available_models = models.read().cloned().unwrap_or_default();
    let health = runtime_health.read().cloned();
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
                    div { class: "flex flex-col gap-4 md:flex-row md:items-start md:justify-between",
                        div {
                            h2 { class: "text-4xl font-extrabold tracking-tight text-slate-900 mb-3",
                                "Settings"
                            }
                            p { class: "text-lg text-slate-600 max-w-2xl",
                                "Tailor your AI experience. Manage local models, agent constraints, and application preferences."
                            }
                        }
                        button {
                            class: "inline-flex items-center gap-2 self-start rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-semibold text-slate-700 shadow-sm hover:bg-slate-50 transition-all",
                            onclick: move |_| {
                                settings.restart();
                                models.restart();
                                runtime_health.restart();
                            },
                            MaterialIcon { name: "refresh", size: 18 }
                            "Refresh Health"
                        }
                    }
                }

                div { class: "space-y-12",
                    section {
                        div { class: "flex items-center gap-3 mb-8",
                            div { class: "p-2 bg-emerald-100 rounded-lg text-emerald-600",
                                MaterialIcon { name: "monitor_heart", size: 24 }
                            }
                            div {
                                h3 { class: "text-xl font-bold text-slate-900", "Runtime Health" }
                                p { class: "text-sm text-slate-500", "Verify that Ollama, your selected model, and local dependencies are ready." }
                            }
                        }

                        if let Some(health) = health {
                            div { class: "grid gap-4 md:grid-cols-2",
                                div { class: "p-5 bg-white rounded-2xl border border-slate-200 shadow-sm space-y-2",
                                    div { class: "flex items-center justify-between gap-3",
                                        p { class: "text-sm font-bold text-slate-900", "Ollama" }
                                        span { class: format!(
                                            "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                            if health.ollama_reachable { "bg-emerald-50 text-emerald-600" } else { "bg-red-50 text-red-600" }
                                        ),
                                            if health.ollama_reachable { "reachable" } else { "offline" }
                                        }
                                    }
                                    p { class: "text-xs text-slate-500", "Endpoint: {health.ollama_endpoint}" }
                                    if health.ollama_reachable {
                                        p { class: "text-xs text-slate-500", "{health.available_models.len()} model(s) discovered from Ollama." }
                                    } else {
                                        p { class: "text-xs text-red-600", "Start Ollama, then refresh this panel." }
                                    }
                                }

                                div { class: "p-5 bg-white rounded-2xl border border-slate-200 shadow-sm space-y-2",
                                    div { class: "flex items-center justify-between gap-3",
                                        p { class: "text-sm font-bold text-slate-900", "Selected Runtime Model" }
                                        span { class: format!(
                                            "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                            if health.runtime_model_available { "bg-emerald-50 text-emerald-600" } else { "bg-amber-50 text-amber-700" }
                                        ),
                                            if health.runtime_model_available { "ready" } else { "missing" }
                                        }
                                    }
                                    code { class: "block bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 text-slate-700 break-all text-sm", "{health.selected_runtime_model}" }
                                    if !health.runtime_model_available {
                                        p { class: "text-xs text-amber-700", "Pick one of the installed Ollama models above or pull this model first." }
                                    }
                                }

                                div { class: "p-5 bg-white rounded-2xl border border-slate-200 shadow-sm space-y-2",
                                    div { class: "flex items-center justify-between gap-3",
                                        p { class: "text-sm font-bold text-slate-900", "Python" }
                                        span { class: format!(
                                            "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                            if health.python_command.is_some() { "bg-emerald-50 text-emerald-600" } else { "bg-red-50 text-red-600" }
                                        ),
                                            if health.python_command.is_some() { "found" } else { "missing" }
                                        }
                                    }
                                    code { class: "block bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 text-slate-700 break-all text-sm",
                                        "{health.python_command.clone().unwrap_or_else(|| \"python (not found)\".to_string())}"
                                    }
                                }

                                div { class: "p-5 bg-white rounded-2xl border border-slate-200 shadow-sm space-y-2",
                                    div { class: "flex items-center justify-between gap-3",
                                        p { class: "text-sm font-bold text-slate-900", "protoc" }
                                        span { class: format!(
                                            "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                            if health.protoc_path.is_some() { "bg-emerald-50 text-emerald-600" } else { "bg-red-50 text-red-600" }
                                        ),
                                            if health.protoc_path.is_some() { "found" } else { "missing" }
                                        }
                                    }
                                    code { class: "block bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 text-slate-700 break-all text-sm",
                                        "{health.protoc_path.clone().unwrap_or_else(|| \"protoc (not found)\".to_string())}"
                                    }
                                }
                            }

                            div { class: "mt-4 p-5 bg-white rounded-2xl border border-slate-200 shadow-sm space-y-3",
                                div { class: "flex items-center justify-between gap-3",
                                    p { class: "text-sm font-bold text-slate-900", "Utility Model" }
                                    span { class: format!(
                                        "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                        if health.title_model_available { "bg-emerald-50 text-emerald-600" } else { "bg-amber-50 text-amber-700" }
                                    ),
                                        if health.title_model_available { "ready" } else { "missing" }
                                    }
                                }
                                code { class: "block bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 text-slate-700 break-all text-sm", "{health.title_model}" }
                            }
                        } else {
                            div { class: "p-6 bg-white rounded-2xl border border-slate-200 shadow-sm text-sm text-slate-500 flex items-center gap-3",
                                div { class: "size-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" }
                                "Checking local runtime health..."
                            }
                        }
                    }

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
