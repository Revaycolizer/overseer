use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::backend::{update_planner, get_available_models};
use crate::db::{Planner, Database};
use crate::state::use_global_state;

#[component]
pub fn PlannerDetails(
    mut selected_planner: Signal<Option<Planner>>,
) -> Element {
    let state = use_global_state();
    
    let mut models = use_signal(|| vec!["loading...".to_string()]);
    let _ = use_resource(move || async move {
        let fetched_models: Vec<String> = get_available_models().await;
        if !fetched_models.is_empty() {
            models.set(fetched_models);
        }
    });

    let p = selected_planner();
    
    let mut local_instructions = use_signal(|| p.as_ref().and_then(|sup| sup.instructions.clone()).unwrap_or_default());
    let mut local_code = use_signal(|| p.as_ref().and_then(|sup| sup.code.clone()).unwrap_or_default());
    let mut is_fullscreen = use_signal(|| false);

    use_effect(move || {
        if let Some(sup) = selected_planner() {
            local_instructions.set(sup.instructions.clone().unwrap_or_default());
            local_code.set(sup.code.clone().unwrap_or_default());
        }
    });

    rsx! {
        div { class: "w-full bg-[#F8FAFC] overflow-y-auto p-8 custom-scrollbar h-full",
            if let Some(mut planner) = selected_planner() {
                div { class: "max-w-4xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500",
                    
                    // Hero Section
                    div { class: "bg-white p-8 rounded-2xl border border-[#E2E8F0] shadow-soft space-y-6",
                        div { class: "flex items-start justify-between",
                            div { class: "flex items-center gap-6",
                                div { class: "w-16 h-16 bg-indigo-50 rounded-2xl flex items-center justify-center border border-indigo-100 shadow-sm",
                                    MaterialIcon { name: "architecture", color: MaterialIconColor::Custom("#6366f1".into()), size: 32 }
                                }
                                div { class: "space-y-1",
                                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Planner Strategy" }
                                    input {
                                        class: "text-2xl font-extrabold text-[#0F172A] leading-tight bg-transparent border-none focus:ring-1 focus:ring-indigo-100 rounded p-1 -ml-1 w-full",
                                        value: "{planner.name}",
                                        oninput: {
                                            let mut p = planner.clone();
                                            move |e| {
                                                p.name = e.value();
                                                selected_planner.set(Some(p.clone()));
                                            }
                                        }
                                    }
                                    input {
                                        class: "text-slate-500 w-full bg-transparent border-none focus:ring-1 focus:ring-indigo-100 rounded p-1 -ml-1",
                                        value: planner.description.clone().unwrap_or_default(),
                                        placeholder: "Add a strategy description...",
                                        oninput: {
                                            let mut p = planner.clone();
                                            move |e| {
                                                p.description = Some(e.value());
                                                selected_planner.set(Some(p.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "flex flex-col items-end gap-2",
                                div { class: "flex items-center gap-2",
                                    span { class: "px-2 py-0.5 bg-slate-100 text-slate-500 rounded text-[9px] font-bold tracking-widest", "v{planner.version.clone().unwrap_or_else(|| \"1.0.0\".into())}" }
                                    span { class: "px-3 py-1 bg-indigo-50 text-indigo-600 rounded-full text-[10px] font-bold uppercase tracking-widest", "Dynamic" }
                                }
                                div {
                                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest text-right mb-1", "Domain" }
                                    input {
                                        class: "text-[10px] font-bold text-slate-500 bg-slate-100 rounded px-2 py-0.5 border-none focus:ring-0 text-right w-24",
                                        value: "{planner.domain}",
                                        oninput: {
                                            let mut p = planner.clone();
                                            move |e| {
                                                p.domain = e.value();
                                                selected_planner.set(Some(p.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-8 pt-4 border-t border-slate-50",
                            div {
                                label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3", "Planning Model" }
                                div { class: "relative group",
                                    select {
                                        class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700 appearance-none focus:ring-2 focus:ring-indigo-500/10 focus:border-indigo-500 transition-all cursor-pointer",
                                        value: planner.model.clone().unwrap_or_default(),
                                        onchange: {
                                            let mut p = planner.clone();
                                            move |e| {
                                                p.model = Some(e.value());
                                                selected_planner.set(Some(p.clone()));
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
                            }

                            div {
                                label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3", "Version" }
                                input {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700 focus:ring-2 focus:ring-indigo-500/10 focus:border-indigo-500 transition-all",
                                    value: planner.version.clone().unwrap_or_else(|| "1.0.0".into()),
                                    oninput: {
                                        let mut p = planner.clone();
                                        move |e| {
                                            p.version = Some(e.value());
                                            selected_planner.set(Some(p.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Preamble Editor
                    div { class: "space-y-4",
                        div { class: "flex justify-between items-center mb-1",
                            label { class: "text-sm font-semibold text-slate-400 uppercase tracking-wider", "Planning Preamble" }
                        }
                        div { class: "code-editor-container min-h-[300px] rounded-2xl overflow-hidden border border-[#E2E8F0] shadow-soft",
                            textarea { 
                                class: "code-editor-textarea",
                                placeholder: "Define the planning logic and strategy guidelines...",
                                value: "{local_instructions}",
                                oninput: {
                                    let mut p = planner.clone();
                                    move |e| {
                                        local_instructions.set(e.value());
                                        p.instructions = Some(e.value());
                                        selected_planner.set(Some(p.clone()));
                                    }
                                },
                                spellcheck: false,
                            }
                            pre { 
                                class: "code-editor-highlight language-markdown",
                                code { "{local_instructions}\n\n" }
                            }
                        }
                    }

                    // Code / Logic Editor
                    div { class: "space-y-4",
                        div { class: "flex justify-between items-center mb-1",
                            label { class: "text-sm font-semibold text-slate-400 uppercase tracking-wider", "Workflow Code (Modular Logic)" }
                        }
                        div { class: "code-editor-container min-h-[400px] rounded-2xl overflow-hidden border border-[#E2E8F0] shadow-soft bg-[#1E1E1E]",
                            textarea { 
                                class: "code-editor-textarea",
                                placeholder: "// Implement custom planning logic or hooks...",
                                value: "{local_code}",
                                oninput: {
                                    let mut p = planner.clone();
                                    move |e| {
                                        local_code.set(e.value());
                                        p.code = Some(e.value());
                                        selected_planner.set(Some(p.clone()));
                                    }
                                },
                                spellcheck: false,
                            }
                            pre { 
                                class: "code-editor-highlight language-rust",
                                code { "{local_code}\n\n" }
                            }
                        }
                    }

                    // Save Footer
                    div { class: "px-8 py-6 bg-white border border-[#E2E8F0] rounded-2xl shadow-soft flex justify-between items-center",
                        div { class: "flex items-center gap-2 text-slate-400",
                            MaterialIcon { name: "auto_awesome_motion", size: 18, color: MaterialIconColor::Custom("#6366f1".into()) }
                            span { class: "text-[10px] font-bold uppercase tracking-widest", "Dynamic Orchestration Active" }
                        }
                        button { 
                            class: "bg-slate-900 text-white px-8 py-3 rounded-xl text-xs font-bold hover:bg-slate-800 transition-all shadow-md active:scale-95 flex items-center gap-2",
                            onclick: move |_| {
                                let mut p = planner.clone();
                                p.instructions = Some(local_instructions());
                                p.code = Some(local_code());
                                let state = state;
                                spawn(async move {
                                    let db = state.read().db.clone();
                                    let _ = update_planner(&db, p).await;
                                });
                            },
                            MaterialIcon { name: "save", size: 16 }
                            " Save & Deploy Planner "
                        }
                    }
                }
            } else {
                div { class: "h-full flex flex-col items-center justify-center text-center space-y-6",
                    div { class: "w-24 h-24 rounded-full bg-white border border-[#E2E8F0] shadow-soft flex items-center justify-center text-5xl", "🏗️" }
                    div {
                        h2 { class: "text-2xl font-bold text-[#0F172A]", "Planners Workshop" }
                        p { class: "text-slate-400 mt-2 max-w-xs", "Select a domain planner to configure its strategy and underlying code logic." }
                    }
                }
            }
        }
    }
}
