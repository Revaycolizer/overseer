use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::backend;

#[component]
pub fn AgentsWorkshop() -> Element {
    let mut global_state = crate::state::use_global_state();
    let db = global_state.read().db.clone();
    
    let mut agents_resource = use_resource::<Vec<crate::engine::utils::discovery::AgentInfo>, _>(move || {
        let db = db.clone();
        async move {
            backend::get_agents(&db).await.unwrap_or_default()
        }
    });

    let agents = agents_resource.read().cloned().unwrap_or_default();
    let mut models = use_signal(|| vec!["loading...".to_string()]);
    let mut selected_agent_id = use_signal(|| agents.first().map(|a| a.id.clone()).unwrap_or_default());
    
    let _ = use_resource(move || async move {
        let fetched_models = backend::list_ollama_models().await;
        if !fetched_models.is_empty() {
            models.set(fetched_models);
        }
    });

    let selected_agent = agents.iter().find(|a| a.id == selected_agent_id()).cloned();
    let mut local_instructions = use_signal(|| selected_agent.as_ref().map(|a| a.instructions.clone()).unwrap_or_default());

    // Sync instructions when selection changes
    let agents_for_effect = agents.clone();
    use_effect(move || {
        if let Some(agent) = agents_for_effect.iter().find(|a| a.id == selected_agent_id()) {
            local_instructions.set(agent.instructions.clone());
        }
    });
    
    // Calculate priority tool for the selected agent
    let priority_tool = selected_agent.as_ref()
        .and_then(|a| a.tools.first())
        .map(|s| s.as_str())
        .unwrap_or("primary");

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
                                                div { class: "flex items-center gap-4",
                                                    div {
                                                        label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-1", "Name" }
                                                        input {
                                                            class: "text-2xl font-extrabold text-slate-900 leading-tight bg-transparent border-none focus:ring-0 p-0",
                                                            value: "{agent.name}",
                                                            oninput: {
                                                                let agent_id = agent.id;
                                                                let mut agents_resource = agents_resource.clone();
                                                                let db = db.clone();
                                                                move |e| {
                                                                    let new_name = e.value();
                                                                    let db = db.clone();
                                                                    let mut agents_resource = agents_resource.clone();
                                                                    let mut updated_agent = agent.clone();
                                                                    updated_agent.name = new_name;
                                                                    spawn(async move {
                                                                        let _ = backend::update_agent(&db, updated_agent).await;
                                                                        agents_resource.restart();
                                                                    });
                                                                }
                                                            }
                                                        }
                                                        p { class: "text-slate-500 mt-1", "{agent.description}" }
                                                    }
                                                }
                                            }
                                            div { class: "flex items-center gap-2",
                                                div { class: "flex flex-col items-end",
                                                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-1", "Domain" }
                                                    input {
                                                        class: "text-[10px] font-bold text-slate-500 bg-slate-100 rounded px-2 py-0.5 border-none focus:ring-0 text-right w-24",
                                                        value: "{agent.domain}",
                                                        oninput: {
                                                            let agent_id = agent.id;
                                                            let mut agents_resource = agents_resource.clone();
                                                            let db = db.clone();
                                                            move |e| {
                                                                let new_domain = e.value();
                                                                let db = db.clone();
                                                                let mut agents_resource = agents_resource.clone();
                                                                let mut updated_agent = agent.clone();
                                                                updated_agent.domain = new_domain;
                                                                spawn(async move {
                                                                    let _ = backend::update_agent(&db, updated_agent).await;
                                                                    agents_resource.restart();
                                                                });
                                                            }
                                                        }
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
                                                    value: global_state.read().agent_models.get(&agent.id.to_string()).cloned().unwrap_or_default(),
                                                    onchange: {
                                                        let agent_id = agent.id;
                                                        move |e| {
                                                            let val = e.value();
                                                            global_state.write().agent_models.insert(agent_id.to_string(), val);
                                                        }
                                                    },
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
                                                        for tool in backend::get_tools(&db).await.unwrap_or_default() {
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

                                div { class: "p-8",
                                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-4", "Advanced Context & Directives" }
                                    div { class: "bg-slate-900 rounded-xl p-6 font-mono text-[13px] text-slate-300 relative",
                                        textarea {
                                            class: "w-full bg-transparent border-none focus:ring-0 text-slate-300 resize-none min-h-[150px] custom-scrollbar",
                                            value: "{local_instructions}",
                                            oninput: move |e| local_instructions.set(e.value()),
                                        }
                                    }
                                }

                                div { class: "px-8 py-5 bg-slate-50 border-t border-slate-100 flex justify-between items-center rounded-b-xl",
                                    div { class: "flex items-center gap-2 text-slate-400",
                                        MaterialIcon { name: "history", size: 14 }
                                        span { class: "text-[10px] font-bold", "Last updated: Today, 14:22" }
                                    }
                                    button { 
                                        class: "bg-slate-900 text-white px-6 py-2 rounded-lg text-xs font-bold hover:bg-slate-800 transition-all shadow-md active:scale-95",
                                        onclick: {
                                            let agent = agent.clone();
                                            let local_instructions = local_instructions.clone();
                                            let db = global_state.read().db.clone();
                                            move |_| {
                                                let mut updated_agent = agent.clone();
                                                updated_agent.instructions = local_instructions();
                                                let db = db.clone();
                                                spawn(async move {
                                                    let _ = backend::update_agent(&db, updated_agent).await;
                                                    agents_resource.restart();
                                                });
                                            }
                                        },
                                        " Save Configuration "
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
