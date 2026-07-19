use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn AgentsWorkshop() -> Element {
    let global_state = crate::state::use_global_state();
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
        use_signal(|| agents.first().map(|a| a.id).unwrap_or_default());
    let mut selected_version_tag = use_signal(String::new);

    let _ = use_resource(move || async move {
        let fetched_models: Vec<String> = backend::get_available_models().await;
        if !fetched_models.is_empty() {
            models.set(fetched_models);
        }
    });

    let selected_agent = agents.iter().find(|a| a.id == selected_agent_id()).cloned();
    let mut local_name = use_signal(|| selected_agent.as_ref().map(|a| a.name.clone()).unwrap_or_default());
    let mut local_description = use_signal(|| selected_agent.as_ref().map(|a| a.description.clone()).unwrap_or_default());
    let mut local_domain = use_signal(|| selected_agent.as_ref().map(|a| a.domain.clone()).unwrap_or_default());
    let mut local_model = use_signal(|| selected_agent.as_ref().map(|a| a.model.clone()).unwrap_or_default());
    let mut local_instructions = use_signal(|| selected_agent.as_ref().map(|a| a.instructions.clone()).unwrap_or_default());
    let mut status = use_signal(String::new);

    // Sync editor fields when the selection changes.
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
    let version_history_resource = use_resource(move || {
        let db = db_for_versions.clone();
        let agent_id = selected_agent_id();
        async move {
            if agent_id != 0 {
                backend::get_version_history(&db, "agent", agent_id).await.unwrap_or_default()
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
                backend::get_version_content(&db, "agent", agent_id, &version_tag).await.unwrap_or_default()
            } else {
                None
            }
        }
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

    rsx! {
        main { class: "flex-1 overflow-y-auto custom-scrollbar bg-white",
            div { class: "max-w-6xl mx-auto w-full px-6 py-8",
                div { class: "flex items-center justify-between mb-6",
                    h1 { class: "text-xl font-semibold text-slate-900", "Agents" }
                    button {
                        class: "text-sm font-medium text-slate-600 hover:text-slate-900 flex items-center gap-1",
                        onclick: {
                            let db = db.clone();
                            move |_| {
                                let db = db.clone();
                                spawn(async move {
                                    let _ = backend::add_agent(&db, "New Agent", "A new custom agent", "core").await;
                                    agents_resource.restart();
                                });
                            }
                        },
                        MaterialIcon { name: "add", size: 16 }
                        "New agent"
                    }
                }

                div { class: "flex gap-8 items-start",
                    // Left: plain agent list
                    div { class: "w-56 shrink-0",
                        for agent in agents.clone().into_iter() {
                            button {
                                key: "{agent.id}",
                                class: if selected_agent_id() == agent.id { "w-full text-left px-3 py-2 rounded-lg bg-slate-100 mb-0.5" } else { "w-full text-left px-3 py-2 rounded-lg hover:bg-slate-50 mb-0.5" },
                                onclick: {
                                    let id = agent.id;
                                    move |_| selected_agent_id.set(id)
                                },
                                div { class: "text-sm font-medium text-slate-800 truncate", "{agent.name}" }
                                div { class: "text-xs text-slate-400 truncate", "{agent.domain}" }
                            }
                        }
                    }

                    // Right: simple editor
                    div { class: "flex-1 min-w-0",
                        if let Some(agent) = selected_agent.clone() {
                            div { class: "space-y-6 max-w-2xl",
                                Field { label: "Name",
                                    input {
                                        class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-900 focus:outline-none focus:border-slate-400",
                                        value: "{local_name}",
                                        oninput: move |e| local_name.set(e.value()),
                                    }
                                }
                                Field { label: "Description",
                                    input {
                                        class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-700 focus:outline-none focus:border-slate-400",
                                        value: "{local_description}",
                                        oninput: move |e| local_description.set(e.value()),
                                    }
                                }
                                div { class: "grid grid-cols-2 gap-4",
                                    Field { label: "Domain",
                                        input {
                                            class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-700 focus:outline-none focus:border-slate-400",
                                            value: "{local_domain}",
                                            oninput: move |e| local_domain.set(e.value()),
                                        }
                                    }
                                    Field { label: "Model",
                                        select {
                                            class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm text-slate-700 bg-white focus:outline-none focus:border-slate-400",
                                            value: "{local_model}",
                                            onchange: move |e| local_model.set(e.value()),
                                            for model in models() {
                                                option { value: "{model}", "{model}" }
                                            }
                                        }
                                    }
                                }

                                // Tools — plain inline chips
                                Field { label: "Tools",
                                    div { class: "flex flex-wrap gap-1.5 items-center",
                                        for tool_name in agent.tools.clone() {
                                            span { class: "inline-flex items-center gap-1 border border-slate-200 rounded-md pl-2 pr-1 py-0.5 text-xs text-slate-700",
                                                "{tool_name}"
                                                button {
                                                    class: "text-slate-300 hover:text-red-500",
                                                    onclick: {
                                                        let agent_id = agent.id;
                                                        let tool_name = tool_name.clone();
                                                        let db = db.clone();
                                                        move |_| {
                                                            let db = db.clone();
                                                            let tool_name = tool_name.clone();
                                                            spawn(async move {
                                                                if let Ok(tools) = backend::get_tools(&db).await {
                                                                    if let Some(tool) = tools.iter().find(|t| t.name == tool_name) {
                                                                        let _ = backend::remove_tool_from_agent(&db, agent_id, tool.id).await;
                                                                        agents_resource.restart();
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },
                                                    MaterialIcon { name: "close", size: 12 }
                                                }
                                            }
                                        }
                                        // Add-tool inline select
                                        select {
                                            class: "border border-dashed border-slate-300 rounded-md px-2 py-0.5 text-xs text-slate-500 bg-white",
                                            onchange: {
                                                let agent_id = agent.id;
                                                let db = db.clone();
                                                move |e| {
                                                    let val = e.value();
                                                    if val.is_empty() { return; }
                                                    let db = db.clone();
                                                    spawn(async move {
                                                        if let Ok(tools) = backend::get_tools(&db).await {
                                                            if let Some(tool) = tools.iter().find(|t| t.name == val) {
                                                                let _ = backend::add_tool_to_agent(&db, agent_id, tool.id).await;
                                                                agents_resource.restart();
                                                            }
                                                        }
                                                    });
                                                }
                                            },
                                            option { value: "", "+ add tool" }
                                            for tool in tools_resource.read().cloned().unwrap_or_default() {
                                                if !agent.tools.contains(&tool.name) {
                                                    option { value: "{tool.name}", "{tool.name}" }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Instructions — plain editor
                                Field { label: "Instructions",
                                    textarea {
                                        class: "w-full min-h-[320px] border border-slate-200 rounded-lg px-3 py-2 text-[13px] font-mono leading-relaxed text-slate-800 focus:outline-none focus:border-slate-400 resize-y",
                                        value: "{local_instructions}",
                                        oninput: move |e| local_instructions.set(e.value()),
                                        spellcheck: false,
                                    }
                                }

                                // Actions
                                div { class: "flex items-center gap-3",
                                    button {
                                        class: if agent_dirty { "bg-slate-900 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-slate-800" } else { "bg-slate-200 text-slate-400 px-4 py-2 rounded-lg text-sm font-medium cursor-default" },
                                        disabled: !agent_dirty,
                                        onclick: {
                                            let agent = agent.clone();
                                            let db = db.clone();
                                            move |_| {
                                                let db = db.clone();
                                                let mut updated = agent.clone();
                                                updated.name = local_name();
                                                updated.description = local_description();
                                                updated.domain = local_domain();
                                                updated.model = local_model();
                                                updated.instructions = local_instructions();
                                                status.set("Saving…".to_string());
                                                spawn(async move {
                                                    let _ = backend::update_agent(&db, updated).await;
                                                    agents_resource.restart();
                                                    status.set("Saved.".to_string());
                                                });
                                            }
                                        },
                                        "Save"
                                    }
                                    button {
                                        class: "border border-slate-200 text-slate-600 px-4 py-2 rounded-lg text-sm font-medium hover:bg-slate-50",
                                        onclick: {
                                            let agent_id = agent.id;
                                            let db = db.clone();
                                            let version_history_resource = version_history_resource.clone();
                                            move |_| {
                                                let db = db.clone();
                                                let mut version_history_resource = version_history_resource.clone();
                                                spawn(async move {
                                                    let tag = format!("snapshot-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
                                                    let _ = backend::publish_agent_version(&db, agent_id, &tag, "Manual snapshot").await;
                                                    version_history_resource.restart();
                                                    status.set("Snapshot saved.".to_string());
                                                });
                                            }
                                        },
                                        "Snapshot"
                                    }
                                    if !status().is_empty() {
                                        span { class: "text-xs text-slate-400", "{status}" }
                                    }
                                }

                                // Version history — compact, collapsible
                                details { class: "border-t border-slate-100 pt-4",
                                    summary { class: "text-xs font-medium text-slate-500 cursor-pointer select-none", "Version history" }
                                    div { class: "mt-3 space-y-1",
                                        for version in version_history_resource.read().cloned().unwrap_or_default() {
                                            div { class: "flex items-center justify-between gap-3 text-xs py-1",
                                                button {
                                                    class: if selected_version_tag() == version.version_tag { "text-slate-900 font-medium text-left" } else { "text-slate-500 hover:text-slate-800 text-left" },
                                                    onclick: {
                                                        let tag = version.version_tag.clone();
                                                        move |_| selected_version_tag.set(tag.clone())
                                                    },
                                                    "{version.version_tag}"
                                                    span { class: "text-slate-300 ml-2", {version.created_at.format("%Y-%m-%d %H:%M").to_string()} }
                                                }
                                                button {
                                                    class: "text-slate-400 hover:text-slate-900",
                                                    onclick: {
                                                        let db = db.clone();
                                                        let agent_id = agent.id;
                                                        let tag = version.version_tag.clone();
                                                        let version_history_resource = version_history_resource.clone();
                                                        move |_| {
                                                            let db = db.clone();
                                                            let tag = tag.clone();
                                                            let mut version_history_resource = version_history_resource.clone();
                                                            spawn(async move {
                                                                let _ = backend::rollback_version(&db, "agent", agent_id, &tag).await;
                                                                agents_resource.restart();
                                                                version_history_resource.restart();
                                                                status.set("Rolled back.".to_string());
                                                            });
                                                        }
                                                    },
                                                    "rollback"
                                                }
                                            }
                                        }
                                        if let Some(content) = version_content_resource.read().cloned().flatten() {
                                            pre { class: "mt-2 max-h-56 overflow-auto custom-scrollbar bg-slate-50 border border-slate-200 rounded-lg p-3 text-[11px] text-slate-700 whitespace-pre-wrap", "{content}" }
                                        }
                                    }
                                }
                            }
                        } else {
                            p { class: "text-sm text-slate-400 py-16", "Select an agent to edit, or create a new one." }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Field(label: &'static str, children: Element) -> Element {
    rsx! {
        div {
            label { class: "block text-xs font-medium text-slate-500 mb-1.5", "{label}" }
            {children}
        }
    }
}
