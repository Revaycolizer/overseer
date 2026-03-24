use crate::db::Supervisor;
use crate::engine::utils::discovery::AgentInfo;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn SupervisorHeroCard(
    mut selected_supervisor: Signal<Option<Supervisor>>,
    models: Signal<Vec<String>>,
    agents: Signal<Vec<AgentInfo>>,
    assigned_agents_count: usize,
    tool_count: usize,
    workbench_status: String,
    save_state: String,
) -> Element {
    let Some(supervisor) = selected_supervisor() else {
        return rsx! {};
    };

    let (state_class, state_label) = match save_state.as_str() {
        "draft" => ("bg-amber-50 text-amber-700 border-amber-100", "Draft"),
        "saving" => ("bg-blue-50 text-blue-700 border-blue-100", "Saving"),
        "error" => ("bg-red-50 text-red-700 border-red-100", "Error"),
        _ => ("bg-emerald-50 text-emerald-700 border-emerald-100", "Synced"),
    };

    rsx! {
        div { class: "bg-white p-8 rounded-2xl border border-slate-200 shadow-soft space-y-6 animate-in fade-in slide-in-from-top-4 duration-500",
            div { class: "flex flex-col xl:flex-row xl:items-start xl:justify-between gap-6",
                div { class: "flex items-start gap-6 min-w-0",
                    div { class: "w-16 h-16 bg-blue-50 text-brand-blue rounded-2xl flex items-center justify-center border border-blue-100 shadow-sm",
                        MaterialIcon { name: "account_tree", size: 30 }
                    }
                    div { class: "space-y-2 min-w-0 flex-1",
                        p { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Supervisor Identity" }
                        input {
                            class: "text-2xl font-extrabold text-slate-900 leading-tight bg-transparent border-none focus:ring-2 focus:ring-brand-blue/10 rounded-lg p-1 -ml-1 w-full transition-all",
                            value: "{supervisor.name}",
                            oninput: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.name = e.value();
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            }
                        }
                        input {
                            class: "text-slate-500 w-full bg-transparent border-none focus:ring-2 focus:ring-brand-blue/10 rounded-lg p-1 -ml-1 transition-all",
                            value: supervisor.description.clone().unwrap_or_default(),
                            placeholder: "Add a description...",
                            oninput: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.description = Some(e.value());
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            }
                        }
                    }
                }

                div { class: "flex flex-wrap items-center gap-2 xl:justify-end",
                    span {
                        class: format!("px-3 py-1 rounded-full text-[10px] font-black uppercase tracking-widest border {}", state_class),
                        "{state_label}"
                    }
                    span { class: "px-3 py-1 bg-slate-100 text-slate-600 rounded-full text-[10px] font-black uppercase tracking-widest border border-slate-200",
                        "Domain: {supervisor.domain}"
                    }
                    span { class: "px-3 py-1 bg-slate-100 text-slate-600 rounded-full text-[10px] font-black uppercase tracking-widest border border-slate-200",
                        "Agents: {assigned_agents_count}"
                    }
                    span { class: "px-3 py-1 bg-slate-100 text-slate-600 rounded-full text-[10px] font-black uppercase tracking-widest border border-slate-200",
                        "Tools: {tool_count}"
                    }
                }
            }

            div { class: "rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3",
                p { class: "text-[10px] font-black uppercase tracking-widest text-slate-400", "Workbench Status" }
                p { class: "text-sm font-semibold text-slate-800 mt-2", "{workbench_status}" }
            }

            div { class: "grid grid-cols-1 md:grid-cols-3 gap-8 pt-2",
                div { class: "space-y-3",
                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Model Assignment" }
                    div { class: "relative",
                        select {
                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2.5 text-sm font-semibold text-slate-700 appearance-none focus:ring-2 focus:ring-brand-blue/10 focus:border-blue-400 transition-all cursor-pointer",
                            value: supervisor.model.clone().unwrap_or_default(),
                            onchange: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.model = Some(e.value());
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            },
                            for model in models() {
                                option { value: "{model}", "{model}" }
                            }
                        }
                        div { class: "absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400",
                            MaterialIcon { name: "unfold_more", size: 16 }
                        }
                    }
                }

                div { class: "space-y-3",
                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Report Style" }
                    div { class: "relative",
                        select {
                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2.5 text-sm font-semibold text-slate-700 appearance-none focus:ring-2 focus:ring-brand-blue/10 focus:border-blue-400 transition-all cursor-pointer",
                            value: supervisor.report_style.clone().unwrap_or_else(|| "detailed".to_string()),
                            onchange: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.report_style = Some(e.value());
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            },
                            option { value: "detailed", "Detailed" }
                            option { value: "concise", "Concise" }
                            option { value: "narrative", "Narrative" }
                        }
                        div { class: "absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400",
                            MaterialIcon { name: "description", size: 16 }
                        }
                    }
                }

                div { class: "space-y-3",
                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Grounding & Priority" }
                    div { class: "flex gap-2",
                        div { class: "flex-1 relative",
                            select {
                                class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2.5 text-sm font-semibold text-slate-700 appearance-none focus:ring-2 focus:ring-brand-blue/10 focus:border-blue-400 transition-all cursor-pointer",
                                value: if let Some(id) = supervisor.verification_agent_id { id.to_string() } else { "0".to_string() },
                                onchange: {
                                    let mut sv = supervisor.clone();
                                    move |e| {
                                        let id = e.value().parse::<i64>().unwrap_or(0);
                                        sv.verification_agent_id = if id > 0 { Some(id) } else { None };
                                        selected_supervisor.set(Some(sv.clone()));
                                    }
                                },
                                option { value: "0", "None (Auto)" }
                                for agent in agents() {
                                    option { value: "{agent.id}", "{agent.name}" }
                                }
                            }
                            div { class: "absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400",
                                MaterialIcon { name: "verified_user", size: 14 }
                            }
                        }
                        input {
                            r#type: "number",
                            class: "w-20 bg-slate-50 border border-slate-200 rounded-xl px-4 py-2.5 text-sm font-semibold text-slate-700 focus:ring-2 focus:ring-brand-blue/10 focus:border-blue-400 transition-all",
                            value: "{supervisor.routing_priority}",
                            oninput: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    if let Ok(p) = e.value().parse::<i32>() {
                                        sv.routing_priority = p;
                                        selected_supervisor.set(Some(sv.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-2 gap-8",
                div { class: "space-y-3",
                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Orchestration Switches" }
                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                        // Direct Pipeline (Bypass)
                        div { class: "flex items-center justify-between p-3 bg-slate-50 border border-slate-200 rounded-xl shadow-sm",
                            div { class: "flex items-center gap-2.5",
                                div { class: "w-8 h-8 rounded-lg bg-blue-50 border border-blue-100 flex items-center justify-center text-brand-blue",
                                    MaterialIcon { name: "speed", size: 16 }
                                }
                                div {
                                    p { class: "text-[11px] font-black text-slate-900 tracking-tight", "Direct Pipeline" }
                                    p { class: "text-[9px] text-slate-400 font-bold", "Bypass synthesis" }
                                }
                            }
                            button {
                                r#type: "button",
                                class: format!(
                                    "relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {}",
                                    if supervisor.bypass_synthesizer { "bg-brand-blue" } else { "bg-slate-300" }
                                ),
                                onclick: {
                                    let mut sv = supervisor.clone();
                                    move |_| {
                                        sv.bypass_synthesizer = !sv.bypass_synthesizer;
                                        selected_supervisor.set(Some(sv.clone()));
                                    }
                                },
                                span {
                                    class: format!(
                                        "pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {}",
                                        if supervisor.bypass_synthesizer { "translate-x-4" } else { "translate-x-0" }
                                    )
                                }
                            }
                        }

                        // Evidence Visualization
                        div { class: "flex items-center justify-between p-3 bg-slate-50 border border-slate-200 rounded-xl shadow-sm",
                            div { class: "flex items-center gap-2.5",
                                div { class: "w-8 h-8 rounded-lg bg-indigo-50 border border-indigo-100 flex items-center justify-center text-indigo-500",
                                    MaterialIcon { name: "visibility_off", size: 16 }
                                }
                                div {
                                    p { class: "text-[11px] font-black text-slate-900 tracking-tight", "Show Evidence" }
                                    p { class: "text-[9px] text-slate-400 font-bold", "Toggle data visualization" }
                                }
                            }
                            button {
                                r#type: "button",
                                class: format!(
                                    "relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {}",
                                    if !supervisor.hide_evidence { "bg-indigo-500" } else { "bg-slate-300" }
                                ),
                                onclick: {
                                    let mut sv = supervisor.clone();
                                    move |_| {
                                        sv.hide_evidence = !sv.hide_evidence;
                                        selected_supervisor.set(Some(sv.clone()));
                                    }
                                },
                                span {
                                    class: format!(
                                        "pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {}",
                                        if !supervisor.hide_evidence { "translate-x-4" } else { "translate-x-0" }
                                    )
                                }
                            }
                        }
                    }
                }

                div { class: "space-y-3",
                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Fast-Path Recognition" }
                    div { class: "relative group",
                        div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-slate-300 group-focus-within:text-brand-blue",
                            MaterialIcon { name: "auto_awesome", size: 16 }
                        }
                        input {
                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl pl-11 pr-4 py-2.5 text-sm font-semibold text-slate-700 focus:ring-2 focus:ring-brand-blue/10 focus:border-blue-400 transition-all placeholder:text-slate-300",
                            placeholder: "Comma-separated keywords (e.g. video, edit, create)",
                            value: supervisor.fast_path_keywords.clone().unwrap_or_default(),
                            oninput: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.fast_path_keywords = Some(e.value());
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
