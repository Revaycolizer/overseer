use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn Settings() -> Element {
    let state = crate::state::use_global_state();
    let nav = use_navigator();
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
    let release_version = env!("CARGO_PKG_VERSION");
    let runtime_ready_count = health
        .as_ref()
        .map(|health| {
            [
                health.ollama_reachable,
                health.runtime_model_available,
                health.protoc_path.is_some(),
                health.title_model_available,
                health.native_runtime,
            ]
            .into_iter()
            .filter(|item| *item)
            .count()
        })
        .unwrap_or(0);
    let runtime_health_summary = if let Some(health) = health.as_ref() {
        if health.ollama_reachable
            && health.runtime_model_available
            && health.protoc_path.is_some()
            && health.native_runtime
        {
            if health.python_command.is_some() {
                "Local runtime is ready for chat, native tools, and optional Python extensions."
                    .to_string()
            } else {
                "Local runtime is ready for chat and native tools. Python is optional for advanced extensions.".to_string()
            }
        } else if !health.ollama_reachable {
            "Ollama is offline. Start it before expecting chat or orchestration to run.".to_string()
        } else if !health.runtime_model_available {
            "Your selected runtime model is not installed yet. Pick an installed model or pull the missing one.".to_string()
        } else if health.protoc_path.is_none() {
            "protoc is missing. Build and generation flows will stay blocked until it is available."
                .to_string()
        } else if !health.native_runtime {
            "The native tool runtime is not ready yet. Restart Overseer and refresh this panel."
                .to_string()
        } else {
            "Runtime health is partially ready. Review the checks below before running deeper workflows.".to_string()
        }
    } else {
        "Checking local runtime health...".to_string()
    };
    let next_step_label = if let Some(health) = health.as_ref() {
        if !health.ollama_reachable {
            "Start Ollama".to_string()
        } else if !health.runtime_model_available {
            "Choose installed model".to_string()
        } else if health.protoc_path.is_none() {
            "Install protoc".to_string()
        } else if !health.native_runtime {
            "Restart native runtime".to_string()
        } else if !health.title_model_available {
            "Pick utility model".to_string()
        } else {
            "Runtime looks healthy".to_string()
        }
    } else {
        "Checking runtime".to_string()
    };

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-slate-50 overflow-y-auto custom-scrollbar",
            main { class: "flex-1 w-full max-w-6xl mx-auto p-6 md:p-10 xl:p-12",
                // Header section
                div { class: "mb-10 border-b border-slate-200 pb-7",
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

                div { class: "space-y-9",
                    section {
                        div { class: "grid gap-4 xl:grid-cols-[minmax(0,1fr)_300px]",
                            div { class: "rounded-3xl bg-white/80 p-6 md:p-8 space-y-6",
                                div { class: "flex items-start justify-between gap-4 flex-wrap",
                                    div { class: "space-y-2 max-w-2xl",
                                        p { class: "text-[11px] font-bold uppercase tracking-[0.24em] text-slate-400",
                                            "Runtime Command Center"
                                        }
                                        h3 { class: "text-2xl font-black tracking-tight text-slate-900",
                                            "See if Overseer is actually ready before you run anything."
                                        }
                                        p { class: "text-sm leading-relaxed text-slate-500",
                                            "{runtime_health_summary}"
                                        }
                                    }
                                    div { class: "rounded-2xl bg-slate-50/80 px-4 py-3 text-right min-w-[120px]",
                                        p { class: "text-[10px] font-bold uppercase tracking-widest text-slate-400",
                                            "Checks Passing"
                                        }
                                        p { class: "text-3xl font-black text-slate-900 leading-none mt-2",
                                            "{runtime_ready_count}/5"
                                        }
                                    }
                                }

                                div { class: "grid gap-4 sm:grid-cols-3",
                                    SummaryTile {
                                        title: "Runtime Model".to_string(),
                                        value: default_chat_model.clone(),
                                        hint: "Primary model for chat, planners, and supervisors".to_string(),
                                        icon: "smart_toy".to_string()
                                    }
                                    SummaryTile {
                                        title: "Utility Model".to_string(),
                                        value: title_model.clone(),
                                        hint: "Used for lighter naming and summary tasks".to_string(),
                                        icon: "auto_awesome".to_string()
                                    }
                                    SummaryTile {
                                        title: "Next Step".to_string(),
                                        value: next_step_label.clone(),
                                        hint: "Most important fix before relying on the system".to_string(),
                                        icon: "flag".to_string()
                                    }
                                }
                            }

                            div { class: "rounded-3xl bg-white/75 p-6 space-y-4",
                                div {
                                    p { class: "text-[11px] font-bold uppercase tracking-[0.24em] text-slate-400",
                                        "First-Run Checklist"
                                    }
                                    h3 { class: "text-lg font-black text-slate-900 mt-2",
                                        "Before you trust a run"
                                    }
                                }
                                if let Some(health) = health.as_ref() {
                                    ChecklistItem {
                                        label: "Ollama reachable".to_string(),
                                        done: health.ollama_reachable,
                                        help: "The local model endpoint must answer before chat can work.".to_string()
                                    }
                                    ChecklistItem {
                                        label: "Runtime model installed".to_string(),
                                        done: health.runtime_model_available,
                                        help: "Your selected default model should exist in Ollama.".to_string()
                                    }
                                    ChecklistItem {
                                        label: "Native tools active".to_string(),
                                        done: health.native_runtime,
                                        help: "Core web, HTTP, and monitoring tools run natively now.".to_string()
                                    }
                                    ChecklistItem {
                                        label: "protoc detected".to_string(),
                                        done: health.protoc_path.is_some(),
                                        help: "Build and generation flows use this binary.".to_string()
                                    }
                                    ChecklistItem {
                                        label: "Utility model ready".to_string(),
                                        done: health.title_model_available,
                                        help: "Use a lighter model for titles and summaries.".to_string()
                                    }
                                } else {
                                    p { class: "text-sm text-slate-500",
                                        "Waiting for runtime checks to finish..."
                                    }
                                }
                            }
                        }
                    }

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
                                HealthCard {
                                    title: "Ollama".to_string(),
                                    status: if health.ollama_reachable { "reachable".to_string() } else { "offline".to_string() },
                                    good: health.ollama_reachable,
                                    body: if health.ollama_reachable {
                                        format!(
                                            "Endpoint: {}. {} model(s) discovered.",
                                            health.ollama_endpoint,
                                            health.available_models.len()
                                        )
                                    } else {
                                        format!(
                                            "Endpoint: {}. Start Ollama, then refresh this panel.",
                                            health.ollama_endpoint
                                        )
                                    },
                                    code: Some(health.ollama_endpoint.clone()),
                                    icon: "hub".to_string()
                                }

                                HealthCard {
                                    title: "Selected Runtime Model".to_string(),
                                    status: if health.runtime_model_available { "ready".to_string() } else { "missing".to_string() },
                                    good: health.runtime_model_available,
                                    body: if health.runtime_model_available {
                                        "This is the default model used when Overseer routes chat and orchestration work.".to_string()
                                    } else {
                                        "Pick one of the installed Ollama models above or pull this model first.".to_string()
                                    },
                                    code: Some(health.selected_runtime_model.clone()),
                                    icon: "smart_toy".to_string()
                                }

                                HealthCard {
                                    title: "Native Tool Runtime".to_string(),
                                    status: if health.native_runtime { "active".to_string() } else { "offline".to_string() },
                                    good: health.native_runtime,
                                    body: "Core Overseer tools now execute in the native runtime instead of depending on Python.".to_string(),
                                    code: Some("rust-native core tools".to_string()),
                                    icon: "terminal".to_string()
                                }

                                HealthCard {
                                    title: "Python Extensions".to_string(),
                                    status: if health.python_command.is_some() { "available".to_string() } else { "optional".to_string() },
                                    good: true,
                                    body: if health.python_command.is_some() {
                                        "Advanced and legacy extension tools can still run through Python when needed.".to_string()
                                    } else {
                                        "Python is not required for core use, but some advanced extension tools will stay unavailable until it is installed.".to_string()
                                    },
                                    code: Some(
                                        health
                                            .python_command
                                            .clone()
                                            .unwrap_or_else(|| "python (optional, not found)".to_string())
                                    ),
                                    icon: "integration_instructions".to_string()
                                }

                                HealthCard {
                                    title: "protoc".to_string(),
                                    status: if health.protoc_path.is_some() { "found".to_string() } else { "missing".to_string() },
                                    good: health.protoc_path.is_some(),
                                    body: "Build generation and protobuf-dependent flows look here first.".to_string(),
                                    code: Some(
                                        health
                                            .protoc_path
                                            .clone()
                                            .unwrap_or_else(|| "protoc (not found)".to_string())
                                    ),
                                    icon: "deployed_code".to_string()
                                }
                            }

                            div { class: "mt-4 grid gap-4 lg:grid-cols-2",
                                HealthCard {
                                    title: "Utility Model".to_string(),
                                    status: if health.title_model_available { "ready".to_string() } else { "missing".to_string() },
                                    good: health.title_model_available,
                                    body: "This lighter model handles conversation titles and small summary tasks.".to_string(),
                                    code: Some(health.title_model.clone()),
                                    icon: "auto_awesome".to_string()
                                }
                                HealthCard {
                                    title: "Runtime Directory".to_string(),
                                    status: "active".to_string(),
                                    good: true,
                                    body: "Overseer stages helper scripts and runtime files in this local directory.".to_string(),
                                    code: Some(health.runtime_dir.clone()),
                                    icon: "folder_managed".to_string()
                                }
                            }
                        } else {
                            div { class: "p-6 rounded-2xl bg-white/80 text-sm text-slate-500 flex items-center gap-3",
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

                        div { class: "grid gap-6 xl:grid-cols-2",
                            div { class: "rounded-2xl bg-white/75 p-6 space-y-5",
                                div {
                                    p { class: "text-[11px] font-bold uppercase tracking-[0.2em] text-slate-400", "Model Selection" }
                                    h4 { class: "text-lg font-black text-slate-900 mt-2", "Choose runtime and utility models" }
                                }
                                div { class: "space-y-4",
                                    div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                                        div { class: "md:pr-4",
                                            label { class: "text-base font-bold text-slate-900 block mb-1",
                                                "Default Runtime Model"
                                            }
                                            p { class: "text-sm text-slate-500",
                                                "Fallback model for chat, planners, supervisors, and new records when a saved model is unavailable."
                                            }
                                        }
                                        select {
                                            class: "bg-white border-slate-200 rounded-xl text-sm w-full md:w-72 focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 cursor-pointer font-medium text-slate-700 transition-all",
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

                                    div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                                        div { class: "md:pr-4",
                                            label { class: "text-base font-bold text-slate-900 block mb-1",
                                                "Utility Model"
                                            }
                                            p { class: "text-sm text-slate-500",
                                                "Lightweight model for titles and summaries."
                                            }
                                        }
                                        select {
                                            class: "bg-white border-slate-200 rounded-xl text-sm w-full md:w-72 focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 cursor-pointer font-medium text-slate-700 transition-all",
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

                            div { class: "rounded-2xl bg-white/75 p-6 space-y-5",
                                div {
                                    p { class: "text-[11px] font-bold uppercase tracking-[0.2em] text-slate-400", "Execution Limits" }
                                    h4 { class: "text-lg font-black text-slate-900 mt-2", "Control runtime boundaries" }
                                }
                                div { class: "space-y-4",
                                    div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                                        div { class: "md:pr-4",
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
                                                class: "bg-white border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 font-medium text-slate-700",
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

                                    div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                                        div { class: "md:pr-4",
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
                                                class: "bg-white border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 font-medium text-slate-700",
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

                                    div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 rounded-2xl bg-slate-50/70 p-4",
                                        div { class: "md:pr-4",
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
                                                class: "bg-white border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 font-medium text-slate-700",
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

                        div { class: "grid gap-6 xl:grid-cols-2",
                            div { class: "group flex items-center justify-between p-6 rounded-2xl bg-white/75 transition-all duration-200",
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

                            div { class: "group flex items-center justify-between p-6 rounded-2xl bg-white/75 transition-all duration-200",
                                div {
                                    p { class: "text-base font-bold text-slate-900 block mb-1",
                                        "Voice Calibration"
                                    }
                                    p { class: "text-sm text-slate-500",
                                        "Train Overseer to recognize your voice for wake-word activation."
                                    }
                                }
                                button { 
                                    class: "bg-blue-50 text-blue-600 px-4 py-2 rounded-xl text-sm font-bold hover:bg-blue-100 transition-all",
                                    onclick: move |_| { nav.push(crate::Route::VoiceTraining {}); },
                                    "Calibrate"
                                }
                            }

                            div { class: "p-6 rounded-2xl bg-white/75",
                                div { class: "mb-4",
                                    p { class: "text-base font-bold text-slate-900 mb-1", "Runtime Diagnostics" }
                                    p { class: "text-sm text-slate-500", "Current local runtime paths used for native tool execution and staging." }
                                }
                                div { class: "grid gap-3 text-sm sm:grid-cols-2",
                                    div {
                                        span { class: "block text-xs font-bold text-slate-400 uppercase tracking-widest mb-1", "Core Runtime" }
                                        code { class: "block rounded-xl bg-slate-900/[0.035] px-3.5 py-2.5 text-xs font-medium tracking-[0.01em] text-slate-700 break-all font-mono", "rust-native tool execution" }
                                    }
                                    div {
                                        span { class: "block text-xs font-bold text-slate-400 uppercase tracking-widest mb-1", "Runtime Directory" }
                                        code { class: "block rounded-xl bg-slate-900/[0.035] px-3.5 py-2.5 text-xs font-medium tracking-[0.01em] text-slate-700 break-all font-mono", "{runtime_dir}" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Footer Branding
                div { class: "mt-20 pt-8 border-t border-slate-200 text-center",
                    p { class: "text-sm font-bold text-slate-400 uppercase tracking-widest",
                        "Overseer v{release_version} - Local runtime command center"
                    }
                }
            }
        }
    }
}

#[component]
fn SummaryTile(title: String, value: String, hint: String, icon: String) -> Element {
    rsx! {
        div { class: "rounded-2xl bg-slate-50/70 px-4 py-4 space-y-2",
            div { class: "flex items-center justify-between gap-3",
                p { class: "text-[10px] font-bold uppercase tracking-widest text-slate-400", "{title}" }
                MaterialIcon { name: "{icon}", size: 16 }
            }
            p { class: "text-base font-black text-slate-900 break-words", "{value}" }
            p { class: "text-xs text-slate-500 leading-relaxed", "{hint}" }
        }
    }
}

#[component]
fn ChecklistItem(label: String, done: bool, help: String) -> Element {
    rsx! {
        div { class: "flex items-start gap-3",
            div {
                class: format!(
                    "mt-0.5 flex h-5 w-5 items-center justify-center rounded-full border {}",
                    if done {
                        "border-emerald-200 bg-emerald-50 text-emerald-600"
                    } else {
                        "border-amber-200 bg-amber-50 text-amber-700"
                    }
                ),
                MaterialIcon {
                    name: if done { "check" } else { "priority_high" },
                    size: 14
                }
            }
            div { class: "space-y-1",
                p { class: "text-sm font-semibold text-slate-900", "{label}" }
                p { class: "text-xs leading-relaxed text-slate-500", "{help}" }
            }
        }
    }
}

#[component]
fn HealthCard(
    title: String,
    status: String,
    good: bool,
    body: String,
    code: Option<String>,
    icon: String,
) -> Element {
    rsx! {
        div { class: "p-5 rounded-2xl bg-white/80 space-y-3",
            div { class: "flex items-center justify-between gap-3",
                div { class: "flex items-center gap-2",
                    MaterialIcon { name: "{icon}", size: 16 }
                    p { class: "text-sm font-bold text-slate-900", "{title}" }
                }
                span {
                    class: format!(
                        "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                        if good {
                            "bg-emerald-50 text-emerald-600"
                        } else if status == "missing" {
                            "bg-amber-50 text-amber-700"
                        } else {
                            "bg-red-50 text-red-600"
                        }
                    ),
                    "{status}"
                }
            }
            p { class: "text-xs leading-relaxed text-slate-500", "{body}" }
            if let Some(code_value) = code {
                code { class: "block rounded-xl bg-slate-900/[0.035] px-3.5 py-2.5 text-xs font-medium tracking-[0.01em] text-slate-700 break-all font-mono", "{code_value}" }
            }
        }
    }
}
