use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn AgentsWorkshop() -> Element {
    let mut global_state = crate::state::use_global_state();
    let db = global_state.read().db.clone();

    let db_for_agents = db.clone();
    let mut agents_resource =
        use_resource::<Vec<crate::engine::utils::discovery::AgentInfo>, _>(move || {
            let db = db_for_agents.clone();
            async move { backend::get_agents(&db).await.unwrap_or_default() }
        });

    let db_for_tools = db.clone();
    let tools_resource =
        use_resource::<Vec<crate::engine::utils::discovery::ToolInfo>, _>(move || {
            let db = db_for_tools.clone();
            async move { backend::get_tools(&db).await.unwrap_or_default() }
        });

    let agents = agents_resource.read().cloned().unwrap_or_default();
    let mut models = use_signal(|| vec!["loading...".to_string()]);
    let mut selected_agent_id =
        use_signal(|| agents.first().map(|a| a.id.clone()).unwrap_or_default());
    let mut selected_version_tag = use_signal(String::new);

    let _ = use_resource(move || async move {
        let fetched_models: Vec<String> = backend::get_available_models().await;
        if !fetched_models.is_empty() {
            models.set(fetched_models);
        }
    });

    let selected_agent = agents.iter().find(|a| a.id == selected_agent_id()).cloned();
    let mut local_name = use_signal(|| {
        selected_agent
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default()
    });
    let mut local_description = use_signal(|| {
        selected_agent
            .as_ref()
            .map(|a| a.description.clone())
            .unwrap_or_default()
    });
    let mut local_domain = use_signal(|| {
        selected_agent
            .as_ref()
            .map(|a| a.domain.clone())
            .unwrap_or_default()
    });
    let mut local_model = use_signal(|| {
        selected_agent
            .as_ref()
            .map(|a| a.model.clone())
            .unwrap_or_default()
    });
    let mut local_instructions = use_signal(|| {
        selected_agent
            .as_ref()
            .map(|a| a.instructions.clone())
            .unwrap_or_default()
    });

    let mut is_fullscreen = use_signal(|| false);
    let mut test_output = use_signal(|| "Run a test to verify agent preamble.".to_string());
    let mut is_testing = use_signal(|| false);
    let mut workbench_status = use_signal(|| "Agent workbench ready.".to_string());

    // Sync instructions when selection changes and handle initial selection
    let agents_for_effect = agents.clone();
    use_effect(move || {
        let current_agents = agents_for_effect.clone();
        if selected_agent_id() == 0 {
            if let Some(first) = current_agents.first() {
                selected_agent_id.set(first.id);
            }
        }

        if let Some(agent) = current_agents.iter().find(|a| a.id == selected_agent_id()) {
            local_name.set(agent.name.clone());
            local_description.set(agent.description.clone());
            local_domain.set(agent.domain.clone());
            local_model.set(agent.model.clone());
            local_instructions.set(agent.instructions.clone());
        }
        selected_version_tag.set(String::new());
    });

    let db_for_versions = db.clone();
    let mut version_history_resource = use_resource(move || {
        let db = db_for_versions.clone();
        let agent_id = selected_agent_id();
        async move {
            if agent_id != 0 {
                backend::get_version_history(&db, "agent", agent_id)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let db_for_version_content = db.clone();
    let version_content_resource = use_resource(move || {
        let db = db_for_version_content.clone();
        let agent_id = selected_agent_id();
        let version_tag = selected_version_tag();
        async move {
            if agent_id != 0 && !version_tag.is_empty() {
                backend::get_version_content(&db, "agent", agent_id, &version_tag)
                    .await
                    .unwrap_or_default()
            } else {
                None
            }
        }
    });

    // Re-highlight when instructions change
    let _inst_trigger = local_instructions();
    use_effect(move || {
        let _ = _inst_trigger;
        let script = "setTimeout(() => { if (window.Prism) { Prism.highlightAll(); } }, 10);";
        document::eval(script);
    });

    let agent_dirty = selected_agent
        .as_ref()
        .map(|agent| {
            agent.name != local_name()
                || agent.description != local_description()
                || agent.domain != local_domain()
                || agent.model != local_model()
                || agent.instructions != local_instructions()
        })
        .unwrap_or(false);

    let domain_label = if local_domain().is_empty() {
        "unassigned".to_string()
    } else {
        local_domain()
    };

    rsx! {
        main { class: "flex-1 overflow-y-auto custom-scrollbar p-8 bg-background-light",
            div { class: "max-w-7xl mx-auto w-full",
                div { class: "mb-10 flex flex-col md:flex-row md:items-end justify-between gap-6",
                    div {
                        h2 { class: "text-3xl font-extrabold tracking-tight text-slate-900 mb-2",
                            "Agents Workshop"
                        }
                        p { class: "text-base text-slate-500 max-w-2xl",
                            " Configure your AI agents, assign specialized models, and manage their capabilities. "
                        }
                    }
                    div { class: "flex items-center gap-3",
                        button { class: "bg-primary text-white px-4 py-2 rounded-lg text-sm font-semibold flex items-center gap-2 hover:bg-slate-800 shadow-sm transition-all active:scale-[0.98]",
                            MaterialIcon { name: "download", color: MaterialIconColor::Light, size: 18 }
                            " Marketplace "
                        }
                    }
                }

                div { class: "grid grid-cols-1 lg:grid-cols-3 gap-8",
                    // Left: Agent List
                    div { class: "lg:col-span-1 space-y-4",
                        h3 { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-4", "Active Agents ({agents.len()})" }
                        for agent in agents.clone().into_iter() {
                            div {
                                class: format!("bg-white border-2 rounded-xl p-4 shadow-sm cursor-pointer transition-all {}", if selected_agent_id() == agent.id { "border-brand-blue shadow-md" } else { "border-slate-100 hover:border-slate-200" }),
                                onclick: {
                                    let agent_id = agent.id.clone();
                                    move |_| selected_agent_id.set(agent_id.clone())
                                },
                                div { class: "flex items-center gap-4",
                                    div { class: "w-10 h-10 bg-slate-50 rounded-lg flex items-center justify-center border border-slate-100",
                                        MaterialIcon {
                                            name: if agent.domain == "stealth" { "visibility_off" } else if agent.domain == "web" { "language" } else { "smart_toy" },
                                            color: MaterialIconColor::Custom("text-slate-500".into()),
                                            size: 20
                                        }
                                    }
                                    div { class: "flex-1",
                                        h4 { class: "font-bold text-slate-900 text-sm", "{agent.name}" }
                                        p { class: "text-[10px] text-slate-400 font-mono uppercase tracking-tighter", "{agent.domain} domain" }
                                    }
                                    div { class: "w-2 h-2 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]" }
                                }
                            }
                        }

                        div {
                            class: "p-4 border-2 border-dashed border-slate-200 rounded-xl flex items-center justify-center gap-3 cursor-pointer hover:border-brand-blue/30 hover:bg-blue-50/20 transition-all",
                            onclick: move |_| {
                                let db = global_state.read().db.clone();
                                spawn(async move {
                                    let _ = backend::add_agent(&db, "New Agent", "A new custom agent", "core").await;
                                    agents_resource.restart();
                                });
                            },
                            MaterialIcon { name: "add", color: MaterialIconColor::Custom("text-slate-400".into()), size: 18 }
                            span { class: "text-xs font-bold text-slate-400", "Custom Agent" }
                        }
                    }

                    // Right: Configuration
                    div { class: "lg:col-span-2",
                        if let Some(agent) = selected_agent.clone() {
                            div { class: "bg-white border border-slate-200 rounded-xl shadow-sm flex flex-col",
                                div { class: "p-8 border-b border-slate-100",
                                    div { class: "flex items-start justify-between mb-8",
                                        div { class: "flex items-center gap-6",
                                            div { class: "w-16 h-16 bg-blue-50 rounded-2xl flex items-center justify-center border border-blue-100 shadow-sm",
                                                MaterialIcon { name: "face", color: MaterialIconColor::Custom("#1e293b".into()), size: 32 }
                                            }
                                            div { class: "space-y-3",
                                                div { class: "flex items-center gap-2 flex-wrap",
                                                    span { class: format!(
                                                        "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                                        if agent_dirty {
                                                            "bg-amber-50 text-amber-700"
                                                        } else {
                                                            "bg-emerald-50 text-emerald-600"
                                                        }
                                                    ),
                                                        if agent_dirty { "Draft Configuration" } else { "Synced" }
                                                    }
                                                    span { class: "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full bg-slate-100 text-slate-600",
                                                        "{domain_label}"
                                                    }
                                                    span { class: "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full bg-blue-50 text-blue-600",
                                                        "{agent.tools.len()} tools"
                                                    }
                                                }
                                                div {
                                                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-1", "Name" }
                                                    input {
                                                        class: "text-2xl font-extrabold text-slate-900 leading-tight bg-transparent border-none focus:ring-1 focus:ring-blue-100 rounded p-1 -ml-1 w-full",
                                                        value: "{local_name}",
                                                        oninput: move |e| local_name.set(e.value())
                                                    }
                                                    textarea {
                                                        class: "w-full text-sm text-slate-500 bg-transparent border border-slate-200 rounded-xl px-3 py-2 focus:ring-1 focus:ring-blue-100 focus:border-blue-200 resize-none",
                                                        rows: "2",
                                                        value: "{local_description}",
                                                        oninput: move |e| local_description.set(e.value())
                                                    }
                                                }
                                            }
                                        }
                                            div { class: "flex items-center gap-2",
                                                div { class: "flex flex-col items-end",
                                                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-1", "Domain" }
                                                    input {
                                                        class: "text-[10px] font-bold text-slate-500 bg-slate-100 rounded px-2 py-0.5 border-none focus:ring-0 text-right w-24",
                                                        value: "{local_domain}",
                                                        oninput: move |e| local_domain.set(e.value())
                                                    }
                                                }
                                                span { class: "px-2 py-0.5 bg-slate-100 text-slate-500 rounded text-[9px] font-bold tracking-widest self-end", "v{agent.version}" }
                                                span { class: "px-3 py-1 bg-blue-50 text-brand-blue rounded-full text-[10px] font-bold uppercase tracking-widest self-end", "Online" }
                                            }
                                    }

                                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-8",
                                        // Model Selection
                                        div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3", "Model Assignment" }
                                            div { class: "relative group",
                                                select {
                                                    class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700 appearance-none focus:ring-2 focus:ring-brand-blue/10 focus:border-brand-blue transition-all cursor-pointer",
                                                    value: "{local_model}",
                                                    onchange: move |e| local_model.set(e.value()),
                                                    for model in models() {
                                                        option { value: "{model}", "{model}" }
                                                    }
                                                }
                                                div { class: "absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400",
                                                    MaterialIcon { name: "unfold_more", size: 18 }
                                                }
                                            }
                                            p { class: "text-[10px] text-slate-400 mt-2 italic", "Local model running via Ollama" }
                                        }

                                        // Capabilities / Tools
                                        div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3", "Enabled Tools" }
                                            div { class: "flex flex-wrap gap-2",
                                                for tool_name in &agent.tools {
                                                    div { class: "bg-white border border-slate-200 px-3 py-1.5 rounded-lg flex items-center gap-2 shadow-sm",
                                                        span { class: "w-1.5 h-1.5 rounded-full bg-blue-400" }
                                                        span { class: "text-xs font-bold text-slate-700", "{tool_name}" }
                                                        button {
                                                            class: "text-slate-300 hover:text-red-400 transition-colors",
                                                            onclick: {
                                                                let agent_id = agent.id;
                                                                let tool_name = tool_name.clone();
                                                                let db = db.clone();
                                                                let mut agents_resource = agents_resource.clone();
                                                                move |_| {
                                                                    let db = db.clone();
                                                                    let mut agents_resource = agents_resource.clone();
                                                                    let tool_name = tool_name.clone();
                                                                    spawn(async move {
                                                                        // Find tool ID first
                                                                        if let Ok(tools) = backend::get_tools(&db).await {
                                                                            if let Some(tool) = tools.iter().find(|t| t.name == tool_name) {
                                                                                let _ = backend::remove_tool_from_agent(&db, agent_id, tool.id).await;
                                                                                agents_resource.restart();
                                                                                workbench_status.set("Tool assignment removed.".to_string());
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            MaterialIcon { name: "close", size: 14 }
                                                        }
                                                    }
                                                }

                                                // Add Tool dropdown/selector
                                                div { class: "relative group",
                                                    button { class: "bg-slate-50 border-2 border-dashed border-slate-200 px-3 py-1.5 rounded-lg flex items-center gap-2 text-slate-400 hover:border-slate-300 transition-all",
                                                        MaterialIcon { name: "add", size: 14 }
                                                        span { class: "text-[10px] font-bold uppercase", "Add Tool" }
                                                    }
                                                    div { class: "hidden group-hover:block absolute top-full left-0 z-50 bg-white border border-slate-100 shadow-xl rounded-xl p-2 min-w-[200px] mt-1",
                                                        // This would ideally be a fetched list of tools not already assigned
                                                        for tool in tools_resource.read().cloned().unwrap_or_default() {
                                                            if !agent.tools.contains(&tool.name) {
                                                                div {
                                                                    class: "px-3 py-2 text-xs font-bold text-slate-600 hover:bg-slate-50 rounded-lg cursor-pointer",
                                                                    onclick: {
                                                                        let agent_id = agent.id;
                                                                        let tool_id = tool.id;
                                                                        let db = db.clone();
                                                                        let mut agents_resource = agents_resource.clone();
                                                                        move |_| {
                                                                            let db = db.clone();
                                                                            let mut agents_resource = agents_resource.clone();
                                                                            spawn(async move {
                                                                                let _ = backend::add_tool_to_agent(&db, agent_id, tool_id).await;
                                                                                agents_resource.restart();
                                                                                workbench_status.set("Tool assignment added.".to_string());
                                                                            });
                                                                        }
                                                                    },
                                                                    "{tool.name}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }


                                div { class: if is_fullscreen() { "fixed inset-0 z-50 bg-[#1e1e1e] flex flex-col p-6 animate-fade-in" } else { "p-8" },
                                    div { class: "flex justify-between items-center mb-4",
                                        label { class: if is_fullscreen() { "block text-sm font-bold text-white mb-0" } else { "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-0" },
                                            "Preamble / System Instructions"
                                        }
                                        div { class: "flex items-center gap-2",
                                            button { class: if is_fullscreen() { "bg-white/10 hover:bg-white/20 text-white p-2 rounded-lg flex items-center" } else { "text-slate-400 hover:text-slate-600 p-1.5 rounded-md hover:bg-slate-50" },
                                                onclick: move |_| is_fullscreen.set(!is_fullscreen()),
                                                if is_fullscreen() {
                                                    MaterialIcon { name: "fullscreen_exit", size: 20, color: MaterialIconColor::Light }
                                                } else {
                                                    MaterialIcon { name: "fullscreen", size: 20 }
                                                }
                                            }
                                        }
                                    }
                                    div { class: if is_fullscreen() { "flex-1 relative overflow-hidden bg-black/20 rounded-xl" } else { "code-editor-container min-h-[350px] rounded-xl overflow-hidden" },
                                        textarea {
                                            class: "code-editor-textarea",
                                            value: "{local_instructions}",
                                            oninput: move |e| local_instructions.set(e.value()),
                                            spellcheck: false,
                                        }
                                        pre {
                                            class: "code-editor-highlight language-markdown",
                                            code {
                                                "{local_instructions}\n\n"
                                            }
                                        }
                                    }
                                    if !is_fullscreen() {
                                        p { class: "text-[10px] text-slate-400 mt-3 italic flex items-center gap-1",
                                            MaterialIcon { name: "info", size: 12 }
                                            " For agents, the preamble serves as their core 'logic' and behavioral code."
                                        }
                                        div { class: "mt-4 bg-[#141414] rounded-xl overflow-hidden shadow-lg border border-white/10",
                                            div { class: "flex items-center justify-between p-3 border-b border-white/5",
                                                div {
                                                    span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                                                        "Pre-Flight Check"
                                                    }
                                                    p { class: "text-[11px] text-slate-500 mt-1",
                                                        "{workbench_status}"
                                                    }
                                                }
                                                div { class: "flex items-center gap-2",
                                                    span { class: format!(
                                                        "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                                        if is_testing() {
                                                            "bg-blue-50 text-blue-600"
                                                        } else if agent_dirty {
                                                            "bg-amber-50 text-amber-700"
                                                        } else {
                                                            "bg-emerald-50 text-emerald-600"
                                                        }
                                                    ),
                                                        if is_testing() { "Running" } else if agent_dirty { "Draft" } else { "Ready" }
                                                    }
                                                    button {
                                                        class: "bg-blue-600 text-white px-3 py-1.5 rounded-lg text-[10px] font-bold hover:bg-blue-700 transition-all shadow-sm active:scale-95 flex items-center gap-2",
                                                        onclick: {
                                                            let instructions = local_instructions.clone();
                                                            move |_| {
                                                                is_testing.set(true);
                                                                test_output.set("Running pre-flight check...".to_string());
                                                                workbench_status.set("Validating agent preamble...".to_string());
                                                                let current_instructions = instructions();
                                                                spawn(async move {
                                                                    match backend::test_agent_preamble(&current_instructions).await {
                                                                        Ok(result) => {
                                                                            test_output.set(result);
                                                                            workbench_status.set("Pre-flight check completed.".to_string());
                                                                        }
                                                                        Err(error) => {
                                                                            test_output.set(format!("Pre-flight Error: {error}"));
                                                                            workbench_status.set("Pre-flight check failed.".to_string());
                                                                        }
                                                                    }
                                                                    is_testing.set(false);
                                                                });
                                                            }
                                                        },
                                                        MaterialIcon { name: "play_arrow", size: 14, color: MaterialIconColor::Light }
                                                        if is_testing() { " Testing... " } else { " Run Test " }
                                                    }
                                                }
                                            }
                                            pre { class: "text-[11px] text-slate-300 font-mono whitespace-pre-wrap p-4 max-h-40 overflow-y-auto custom-scrollbar",
                                                "{test_output}"
                                            }
                                        }
                                    }
                                }

                                div { class: "px-8 pb-8",
                                    div { class: "border border-slate-200 rounded-xl overflow-hidden",
                                        div { class: "px-4 py-3 border-b border-slate-100 bg-slate-50 flex items-center justify-between",
                                            span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Version History" }
                                            if !selected_version_tag().is_empty() {
                                                button {
                                                    class: "bg-slate-900 text-white px-3 py-1.5 rounded-lg text-[10px] font-bold hover:bg-slate-800 transition-all",
                                                    onclick: {
                                                        let db = db.clone();
                                                        let mut agents_resource = agents_resource.clone();
                                                        let mut version_history_resource = version_history_resource.clone();
                                                        let version_tag = selected_version_tag();
                                                        move |_| {
                                                            let db = db.clone();
                                                            let mut agents_resource = agents_resource.clone();
                                                            let mut version_history_resource = version_history_resource.clone();
                                                            let version_tag = version_tag.clone();
                                                            spawn(async move {
                                                                let _ = backend::rollback_version(&db, "agent", agent.id, &version_tag).await;
                                                                agents_resource.restart();
                                                                version_history_resource.restart();
                                                            });
                                                        }
                                                    },
                                                    "Rollback to Snapshot"
                                                }
                                            }
                                        }
                                        div { class: "grid grid-cols-1 xl:grid-cols-[240px_minmax(0,1fr)] gap-4 p-4",
                                            div { class: "space-y-2 max-h-56 overflow-y-auto custom-scrollbar pr-1",
                                                for version in version_history_resource.read().cloned().unwrap_or_default() {
                                                    button {
                                                        key: "{version.id}",
                                                        class: format!(
                                                            "w-full text-left rounded-xl border px-3 py-2 transition-all {}",
                                                            if selected_version_tag() == version.version_tag {
                                                                "border-blue-300 bg-blue-50"
                                                            } else {
                                                                "border-slate-200 bg-slate-50 hover:border-slate-300"
                                                            }
                                                        ),
                                                        onclick: {
                                                            let tag = version.version_tag.clone();
                                                            move |_| selected_version_tag.set(tag.clone())
                                                        },
                                                        div { class: "text-[11px] font-bold text-slate-700", "{version.version_tag}" }
                                                        div { class: "text-[10px] text-slate-400 mt-1", {version.created_at.format("%Y-%m-%d %H:%M").to_string()} }
                                                        div { class: "text-[10px] text-slate-500 mt-1 line-clamp-2", "{version.changelog.clone().unwrap_or_default()}" }
                                                    }
                                                }
                                            }
                                            div { class: "grid grid-cols-1 xl:grid-cols-2 gap-4",
                                                div {
                                                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Current Instructions" }
                                                    pre { class: "min-h-[180px] max-h-[240px] overflow-auto custom-scrollbar bg-slate-50 border border-slate-200 rounded-xl p-3 text-[11px] text-slate-700 whitespace-pre-wrap",
                                                        "{local_instructions}"
                                                    }
                                                }
                                                div {
                                                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Selected Snapshot" }
                                                    pre { class: "min-h-[180px] max-h-[240px] overflow-auto custom-scrollbar bg-slate-50 border border-slate-200 rounded-xl p-3 text-[11px] text-slate-700 whitespace-pre-wrap",
                                                        "{version_content_resource.read().cloned().flatten().unwrap_or_else(|| \"Select a snapshot to compare or rollback.\".to_string())}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "px-8 py-5 bg-slate-50 border-t border-slate-100 flex justify-between items-center rounded-b-xl",
                                    div { class: "flex items-center gap-2 text-slate-400",
                                        MaterialIcon { name: "history", size: 14 }
                                        span { class: "text-[10px] font-bold", "Last updated: Today, 14:22" }
                                    }
                                    div { class: "flex gap-3",
                                        button {
                                            class: "bg-emerald-600 text-white px-6 py-2 rounded-lg text-xs font-bold hover:bg-emerald-700 transition-all shadow-md active:scale-95 flex items-center gap-2",
                                            onclick: {
                                                let agent_id = agent.id;
                                                let db = global_state.read().db.clone();
                                                let mut agents_resource = agents_resource.clone();
                                                move |_| {
                                                    let db = db.clone();
                                                    let mut agents_resource = agents_resource.clone();
                                                    let mut version_history_resource = version_history_resource.clone();
                                                    spawn(async move {
                                                        let version_tag = format!("snapshot-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
                                                        let _ = backend::publish_agent_version(&db, agent_id, &version_tag, "Marketplace release").await;
                                                        agents_resource.restart();
                                                        version_history_resource.restart();
                                                        workbench_status.set("Snapshot published.".to_string());
                                                    });
                                                }
                                            },
                                            MaterialIcon { name: "publish", size: 14 }
                                            " Publish to Marketplace "
                                        }
                                        button {
                                            class: "bg-slate-900 text-white px-6 py-2 rounded-lg text-xs font-bold hover:bg-slate-800 transition-all shadow-md active:scale-95",
                                            onclick: {
                                                let agent = agent.clone();
                                                let local_name = local_name.clone();
                                                let local_description = local_description.clone();
                                                let local_domain = local_domain.clone();
                                                let local_model = local_model.clone();
                                                let local_instructions = local_instructions.clone();
                                                let db = global_state.read().db.clone();
                                                move |_| {
                                                    let db = db.clone();
                                                    let mut updated_agent = agent.clone();
                                                    updated_agent.name = local_name();
                                                    updated_agent.description = local_description();
                                                    updated_agent.domain = local_domain();
                                                    updated_agent.model = local_model();
                                                    updated_agent.instructions = local_instructions();
                                                    workbench_status.set("Saving agent configuration...".to_string());
                                                    spawn(async move {
                                                        let _ = backend::update_agent(&db, updated_agent).await;
                                                        agents_resource.restart();
                                                        workbench_status.set("Agent configuration saved.".to_string());
                                                    });
                                                }
                                            },
                                            " Save & Deploy Preamble "
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "h-full flex flex-col items-center justify-center bg-white border border-slate-200 rounded-xl p-12 text-center",
                                div { class: "w-20 h-20 bg-slate-50 rounded-full flex items-center justify-center mb-6",
                                    MaterialIcon { name: "smart_toy", color: MaterialIconColor::Custom("#cbd5e1".into()), size: 40 }
                                }
                                h4 { class: "text-lg font-bold text-slate-900 mb-2", "No Agent Selected" }
                                p { class: "text-slate-500 max-w-sm", "Select an agent from the list to manage its model configuration and capabilities." }
                            }
                        }
                    }
                }
            }

            footer { class: "py-12 border-t border-slate-200 mt-16 px-8",
                div { class: "max-w-7xl mx-auto flex flex-col md:flex-row justify-between items-center gap-8",
                    div { class: "flex items-center gap-2 text-slate-400",
                        MaterialIcon { name: "verified_user", size: 18 }
                        span { class: "text-[11px] font-bold uppercase tracking-widest",
                            "Secure Local Environment • v2.4.0"
                        }
                    }
                    div { class: "flex gap-10",
                        a {
                            class: "text-[11px] font-bold text-slate-400 hover:text-slate-900 uppercase tracking-widest transition-colors",
                            href: "#",
                            "Documentation"
                        }
                        a {
                            class: "text-[11px] font-bold text-slate-400 hover:text-slate-900 uppercase tracking-widest transition-colors",
                            href: "#",
                            "Privacy Policy"
                        }
                    }
                    div { class: "text-[11px] text-slate-400 font-medium bg-slate-100 px-3 py-1 rounded-full",
                        " Status: Optimal "
                    }
                }
            }
        }
    }
}
