use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::backend::{update_supervisor, get_available_models, get_supervisor_agents, get_agents};
use crate::db::Supervisor;
use crate::state::use_global_state;
use crate::engine::utils::discovery::AgentInfo;

use super::hero_card::SupervisorHeroCard;
use super::preamble_editor::PreambleEditor;
use super::code_editor::CodeEditor;
use super::agent_assignments::AgentAssignments;

#[derive(Clone, PartialEq)]
enum Tab { Preamble, Code, Agents }

impl Tab {
    fn label(&self) -> &'static str {
        match self { Tab::Preamble => "Execution Preamble", Tab::Code => "Orchestration Logic", Tab::Agents => "Agent Assignments" }
    }
    fn icon(&self) -> &'static str {
        match self { Tab::Preamble => "description", Tab::Code => "code", Tab::Agents => "hub" }
    }
}

#[component]
pub fn SupervisorDetails(
    mut selected_supervisor: Signal<Option<Supervisor>>,
    mut supervisors: Resource<Vec<Supervisor>>,
) -> Element {
    let state = use_global_state();

    // ── Resources ────────────────────────────────────────────────────────
    let supervisor = selected_supervisor();
    let supervisor_id = supervisor.as_ref().map(|s| s.id).unwrap_or(0);

    let mut supervisor_agents = use_resource(move || async move {
        let res: Vec<crate::engine::utils::discovery::AgentInfo> = get_supervisor_agents(&state.read().db, supervisor_id).await.unwrap_or_default();
        res
    });

    let agents = use_resource(move || async move {
        let res: Vec<crate::engine::utils::discovery::AgentInfo> = get_agents(&state.read().db).await.unwrap_or_default();
        res
    });

    // ── Available models ───────────────────────────────────────────────────
    let mut models = use_signal(|| vec!["loading...".to_string()]);
    let _ = use_resource(move || async move {
        let fetched: Vec<String> = get_available_models().await;
        if !fetched.is_empty() { models.set(fetched); }
    });

    // ── Local editing signals ──────────────────────────────────────────────
    let mut local_instructions = use_signal(|| {
        supervisor.as_ref().and_then(|sv| sv.instructions.clone()).unwrap_or_default()
    });
    let mut local_code = use_signal(|| {
        supervisor.as_ref().and_then(|sv| sv.code.clone()).unwrap_or_default()
    });

    use_effect(move || {
        if let Some(sv) = selected_supervisor() {
            local_instructions.set(sv.instructions.clone().unwrap_or_default());
            local_code.set(sv.code.clone().unwrap_or_default());
            supervisor_agents.restart();
        }
    });

    let mut active_tab = use_signal(|| Tab::Preamble);

    rsx! {
        div { class: "w-full bg-slate-50 overflow-y-auto p-8 custom-scrollbar h-full",
            if let Some(mut sv_final) = selected_supervisor() {
                div { class: "max-w-4xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500",

                    // ── Hero card ──────────────────────────────────────
                    SupervisorHeroCard {
                        selected_supervisor,
                        models,
                    }

                    // ── Tab bar ────────────────────────────────────────
                    div { class: "flex gap-1 bg-white border border-slate-200 rounded-xl p-1 shadow-soft",
                        for tab in [Tab::Preamble, Tab::Code, Tab::Agents] {
                            {
                                let is_active = active_tab() == tab;
                                let tab_clone = tab.clone();
                                rsx! {
                                    button {
                                        key: "{tab.label()}",
                                        class: format!(
                                            "flex-1 flex items-center justify-center gap-2 px-4 py-2 rounded-lg text-xs font-bold transition-all {}",
                                            if is_active { "bg-brand-blue text-white shadow-md shadow-blue-100" }
                                            else { "text-slate-400 hover:text-slate-600 hover:bg-slate-50" }
                                        ),
                                        onclick: move |_| active_tab.set(tab_clone.clone()),
                                        MaterialIcon {
                                            name: tab.icon(),
                                            size: 16,
                                            color: if is_active { MaterialIconColor::Light } else { MaterialIconColor::Dark }
                                        }
                                        span { class: "uppercase tracking-wider", "{tab.label()}" }
                                    }
                                }
                            }
                        }
                    }

                    // ── Tab content ────────────────────────────────────
                    div { class: "min-h-[400px]",
                        match active_tab() {
                            Tab::Preamble => rsx! {
                                PreambleEditor {
                                    selected_supervisor,
                                    local_instructions,
                                }
                            },
                            Tab::Code => rsx! {
                                CodeEditor {
                                    selected_supervisor,
                                    local_code,
                                }
                            },
                            Tab::Agents => rsx! {
                                AgentAssignments {
                                    supervisor_id: sv_final.id,
                                    supervisor_agents,
                                    agents,
                                }
                            },
                        }
                    }

                    // ── Save footer ────────────────────────────────────
                    div { class: "px-8 py-6 bg-white border border-slate-200 rounded-2xl shadow-soft flex justify-between items-center",
                        div { class: "flex items-center gap-2 text-slate-400",
                            MaterialIcon {
                                name: "verified",
                                size: 18,
                                color: MaterialIconColor::Custom("var(--color-brand-blue)".into())
                            }
                            span { class: "text-[10px] font-bold uppercase tracking-widest", "Infrastructure Synchronization Active" }
                        }
                        div { class: "flex gap-4",
                            button {
                                class: "px-6 py-3 bg-slate-50 hover:bg-slate-100 text-slate-500 rounded-xl text-xs font-bold uppercase tracking-widest transition-all",
                                onclick: move |_| selected_supervisor.set(None),
                                "Close"
                            }
                            button {
                                class: "bg-brand-blue text-white px-8 py-3 rounded-xl text-xs font-bold hover:bg-blue-500 transition-all shadow-md shadow-blue-100 active:scale-95 flex items-center gap-2",
                                onclick: move |_| {
                                    if let Some(mut sv) = selected_supervisor() {
                                        sv.instructions = Some(local_instructions());
                                        sv.code = Some(local_code());
                                        let state = state;
                                        spawn(async move {
                                            let db = state.read().db.clone();
                                            let _ = update_supervisor(&db, sv).await;
                                            supervisors.restart();
                                        });
                                    }
                                },
                                MaterialIcon { name: "save", size: 16 }
                                " Save & Deploy Node "
                            }
                        }
                    }
                }

            } else {
                div { class: "h-full flex flex-col items-center justify-center text-center space-y-6",
                    div { class: "w-24 h-24 rounded-full bg-white border border-slate-200 shadow-soft flex items-center justify-center text-5xl", "🛸" }
                    div {
                        h2 { class: "text-2xl font-bold text-slate-900", "Supervisor Command" }
                        p { class: "text-slate-400 mt-2 max-w-xs", "Select a supervisor node to manage its orchestration logic and agent fleet." }
                    }
                }
            }
        }
    }
}
