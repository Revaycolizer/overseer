use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::backend;

#[component]
pub fn ToolsWorkshop() -> Element {
    let mut global_state = crate::state::use_global_state();
    let db = global_state.read().db.clone();

    let db_resource = db.clone();
    let mut tools_resource = use_resource::<Vec<crate::engine::utils::discovery::ToolInfo>, _>(move || {
        let db_inner = db_resource.clone();
        async move {
            backend::get_tools(&db_inner).await.unwrap_or_default()
        }
    });

    let tools = tools_resource.read().cloned().unwrap_or_default();
    let mut selected_tool_id = use_signal(|| tools.first().map(|t| t.id).unwrap_or(0i64));
    
    let selected_tool = tools.iter().find(|t| t.id == selected_tool_id()).cloned();
    let mut local_code = use_signal(|| selected_tool.as_ref().map(|t| t.code.clone()).unwrap_or_default());

    // Sync code when selection changes
    let tools_for_effect = tools.clone();
    use_effect(move || {
        if let Some(tool) = tools_for_effect.iter().find(|t| t.id == selected_tool_id()) {
            local_code.set(tool.code.clone());
        }
    });

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-background-light overflow-y-auto custom-scrollbar p-8",
            div { class: "max-w-7xl mx-auto w-full",
                div { class: "mb-10 flex flex-col md:flex-row md:items-end justify-between gap-6",
                    div {
                        h2 { class: "text-3xl font-extrabold tracking-tight text-slate-900 mb-2",
                            "Tools Workshop"
                        }
                        p { class: "text-base text-slate-500 max-w-2xl",
                            " Create, test, and deploy functional tools for your AI to interact with the OS. "
                        }
                    }
                    div { class: "flex items-center gap-3",
                        button { 
                            class: "bg-primary text-white px-4 py-2 rounded-lg text-sm font-semibold flex items-center gap-2 hover:bg-slate-800 shadow-sm transition-all active:scale-[0.98]",
                            onclick: {
                                let db_click = db.clone();
                                let mut tools_resource_click = tools_resource.clone();
                                move |_| {
                                    let db_spawn = db_click.clone();
                                    let mut tools_resource_spawn = tools_resource_click.clone();
                                    spawn(async move {
                                        let _ = backend::add_tool(&db_spawn, "New Tool", "A new functional tool", "python").await;
                                        tools_resource_spawn.restart();
                                    });
                                }
                            },
                            MaterialIcon { name: "add", size: 18, color: MaterialIconColor::Light }
                            " New Tool "
                        }
                    }
                }
                div { class: "grid grid-cols-1 lg:grid-cols-3 gap-8",
                    div { class: "lg:col-span-1 space-y-4",
                        div { class: "flex items-center justify-between mb-2",
                            h3 { class: "text-xs font-bold text-slate-400 uppercase tracking-widest",
                                "Local Library"
                            }
                            span { class: "text-[10px] bg-slate-200 text-slate-600 px-2 py-0.5 rounded-full font-bold",
                                "{tools.len()} Total"
                            }
                        }
                        for tool in tools.clone().into_iter() {
                            div { 
                                class: format!("bg-white border-2 rounded-xl p-4 shadow-sm cursor-pointer transition-all {}", if selected_tool_id() == tool.id { "border-brand-blue shadow-md" } else { "border-slate-100 hover:border-slate-200" }),
                                onclick: {
                                    let tool_id = tool.id;
                                    move |_| selected_tool_id.set(tool_id)
                                },
                                div { class: "flex items-center justify-between mb-3",
                                    div { class: "flex items-center gap-3",
                                        div { class: "w-8 h-8 bg-blue-50 text-brand-blue rounded flex items-center justify-center",
                                            MaterialIcon { 
                                                name: "build",
                                                size: 20,
                                                color: MaterialIconColor::Custom("text-brand-blue".into())
                                            }
                                        }
                                        span { class: "font-bold text-slate-900 text-sm",
                                            "{tool.name}"
                                        }
                                    }
                                    span { class: "w-2 h-2 rounded-full bg-emerald-500" }
                                }
                                p { class: "text-xs text-slate-500 line-clamp-1 mb-3",
                                    "{tool.description}"
                                }
                                div { class: "flex items-center gap-2",
                                    span { class: "text-[10px] font-mono bg-slate-100 text-slate-500 px-1.5 py-0.5 rounded",
                                        "{tool.language}"
                                    }
                                    span { class: "text-[10px] text-slate-400", "Local System" }
                                }
                            }
                        }
                    }
                    div { class: "lg:col-span-2 space-y-6",
                        if let Some(ref tool) = selected_tool {
                            div { class: "bg-white border border-slate-200 rounded-xl shadow-sm flex flex-col h-full min-h-[500px]",
                                div { class: "border-b border-slate-200 p-4 flex items-center justify-between",
                                    div { class: "flex items-center gap-4",
                                        div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-1", "Tool Name" }
                                            input { 
                                                class: "font-bold text-slate-900 border-none focus:ring-0 p-0",
                                                value: "{tool.name}",
                                                oninput: {
                                                    let tool_id = tool.id;
                                                    let mut tools_resource = tools_resource.clone();
                                                    let db = db.clone();
                                                    move |e| {
                                                        let new_name = e.value();
                                                        let db = db.clone();
                                                        let mut tools_resource = tools_resource.clone();
                                                        let mut updated_tool = tool.clone();
                                                        updated_tool.name = new_name;
                                                        spawn(async move {
                                                            let _ = backend::update_tool(&db, updated_tool).await;
                                                            tools_resource.restart();
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "flex items-center gap-1",
                                            span { class: "text-[10px] font-bold text-emerald-600 uppercase bg-emerald-50 px-2 py-0.5 rounded",
                                                "Active"
                                            }
                                        }
                                    }
                                    div { class: "flex items-center gap-2",
                                        button { class: "text-slate-400 hover:text-slate-600 p-1.5 rounded-md hover:bg-slate-50",
                                            MaterialIcon { name: "play_arrow", size: 20 }
                                        }
                                        button { 
                                            class: "text-slate-400 hover:text-slate-600 p-1.5 rounded-md hover:bg-slate-50",
                                            onclick: {
                                                let t = tool.clone();
                                                let local_code = local_code.clone();
                                                let db_click = db.clone();
                                                let mut tools_resource_click = tools_resource.clone();
                                                move |_| {
                                                    let mut updated_tool = t.clone();
                                                    updated_tool.code = local_code();
                                                    let db_spawn = db_click.clone();
                                                    let mut tools_resource_spawn = tools_resource_click.clone();
                                                    spawn(async move {
                                                        let _ = backend::update_tool(&db_spawn, updated_tool).await;
                                                        tools_resource_spawn.restart();
                                                    });
                                                }
                                            },
                                            MaterialIcon { name: "save", size: 20 }
                                        }
                                        div { class: "w-[1px] h-4 bg-slate-200 mx-1" }
                                        button { class: "text-slate-400 hover:text-slate-600 p-1.5 rounded-md hover:bg-slate-50",
                                            MaterialIcon { name: "settings", size: 20 }
                                        }
                                    }
                                }
                                div { class: "flex-1 bg-[#1e293b] font-mono text-sm overflow-hidden",
                                    textarea {
                                        class: "w-full h-full bg-transparent border-none focus:ring-0 text-slate-300 p-6 resize-none custom-scrollbar",
                                        value: "{local_code}",
                                        oninput: move |e| local_code.set(e.value()),
                                        spellcheck: false,
                                    }
                                }
                                
                                // Tool Metadata Section
                                div { class: "border-t border-slate-200 p-6 bg-white",
                                    div { class: "grid grid-cols-2 gap-6",
                                        div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Description" }
                                            textarea { 
                                                class: "w-full bg-slate-50 border border-slate-100 rounded-lg p-2 text-xs text-slate-600 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue transition-all min-h-[60px]",
                                                value: "{tool.description}",
                                                oninput: {
                                                    let tool_id = tool.id;
                                                    let mut tools_resource = tools_resource.clone();
                                                    let db = db.clone();
                                                    move |e| {
                                                        let new_desc = e.value();
                                                        let db = db.clone();
                                                        let mut tools_resource = tools_resource.clone();
                                                        let mut updated_tool = tool.clone();
                                                        updated_tool.description = new_desc;
                                                        spawn(async move {
                                                            let _ = backend::update_tool(&db, updated_tool).await;
                                                            tools_resource.restart();
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        div {
                                            label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2", "Script Path" }
                                            input { 
                                                class: "w-full bg-slate-50 border border-slate-100 rounded-lg p-2 text-xs font-mono text-slate-600",
                                                placeholder: "/src/engine/scripts/...",
                                                value: tool.script_path.clone().unwrap_or_default(),
                                                oninput: {
                                                    let tool_id = tool.id;
                                                    let mut tools_resource = tools_resource.clone();
                                                    let db = db.clone();
                                                    move |e| {
                                                        let new_path = e.value();
                                                        let db = db.clone();
                                                        let mut tools_resource = tools_resource.clone();
                                                        let mut updated_tool = tool.clone();
                                                        updated_tool.script_path = if new_path.is_empty() { None } else { Some(new_path) };
                                                        spawn(async move {
                                                            let _ = backend::update_tool(&db, updated_tool).await;
                                                            tools_resource.restart();
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Agent relationships section
                                div { class: "border-t border-slate-200 p-4 bg-slate-50",
                                    h4 { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3", "Used by Agents" }
                                    div { class: "flex flex-wrap gap-2",
                                        {
                                            let agents = backend::get_agents(&db);
                                            rsx! { 
                                                // This is a bit heavy for a loop, but for now it's fine for simple workshops
                                                span { class: "text-[10px] text-slate-400 italic", "Equipped on some system agents" }
                                            }
                                        }
                                    }
                                }

                                div { class: "border-t border-slate-200 p-4 bg-white rounded-b-xl",
                                    div { class: "flex items-center justify-between",
                                        div { class: "flex items-center gap-4",
                                            span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                                                "Console Output"
                                            }
                                            span { class: "text-[10px] text-emerald-600 font-mono",
                                                "Core tool loaded successfully"
                                            }
                                        }
                                        MaterialIcon { name: "keyboard_arrow_up", size: 18, color: MaterialIconColor::Custom("text-slate-300".into()) }
                                    }
                                }
                            }
                        } else {
                            div { class: "flex-1 flex items-center justify-center bg-white border border-slate-200 rounded-xl",
                                p { class: "text-slate-400", "Select a tool to view details" }
                            }
                        }
                    }
                }
                footer { class: "py-12 border-t border-slate-200 mt-12 px-8",
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
                    }
                }
            }
        }
    }
}
