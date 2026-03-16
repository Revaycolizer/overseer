use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::backend::{update_supervisor, get_available_models};
use crate::db::Supervisor;
use crate::engine::utils::discovery::AgentInfo;
use crate::state::use_global_state;

use super::hero_card::SupervisorHeroCard;
use super::preamble_editor::PreambleEditor;
use super::code_editor::CodeEditor;
use super::agent_assignments::AgentAssignments;

// ─── Tab definition ──────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum Tab { Preamble, Code, Agents }

impl Tab {
    fn label(&self) -> &'static str {
        match self { Tab::Preamble => "Preamble", Tab::Code => "Code Logic", Tab::Agents => "Agents" }
    }
    fn icon(&self) -> &'static str {
        match self { Tab::Preamble => "description", Tab::Code => "code", Tab::Agents => "hub" }
    }
}

// ─── Component ───────────────────────────────────────────────────────────────

#[component]
pub fn SupervisorDetails(
    mut selected_supervisor: Signal<Option<Supervisor>>,
    mut supervisor_agents: Resource<Vec<AgentInfo>>,
    agents: Resource<Vec<AgentInfo>>,
) -> Element {
    let state = use_global_state();

    // ── Available models ───────────────────────────────────────────────────
    let mut models = use_signal(|| vec!["loading...".to_string()]);
    let _ = use_resource(move || async move {
        let fetched: Vec<String> = get_available_models().await;
        if !fetched.is_empty() { models.set(fetched); }
    });

    // ── Local editing signals (synced when selection changes) ──────────────
    let s = selected_supervisor();
    let mut local_instructions = use_signal(|| {
        s.as_ref().and_then(|sv| sv.instructions.clone()).unwrap_or_default()
    });
    let mut local_code = use_signal(|| {
        s.as_ref().and_then(|sv| sv.code.clone()).unwrap_or_default()
    });

    use_effect(move || {
        if let Some(sv) = selected_supervisor() {
            local_instructions.set(sv.instructions.clone().unwrap_or_default());
            local_code.set(sv.code.clone().unwrap_or_default());
        }
    });

    // ── Active tab ─────────────────────────────────────────────────────────
    let mut active_tab = use_signal(|| Tab::Preamble);

    // ─────────────────────────────────────────────────────────────────────
    rsx! {
        div { class: "w-full md:w-2/3 bg-[#F8FAFC] overflow-y-auto p-8 custom-scrollbar",

            if let Some(supervisor) = selected_supervisor() {
                div { class: "max-w-4xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500",

                    // ── Hero card ──────────────────────────────────────
                    SupervisorHeroCard {
                        selected_supervisor,
                        models,
                    }

                    // ── Tab bar ────────────────────────────────────────
                    div { class: "flex gap-1 bg-white border border-[#E2E8F0] rounded-xl p-1 shadow-soft",
                        for tab in [Tab::Preamble, Tab::Code, Tab::Agents] {
                            {
                                let is_active = active_tab() == tab;
                                let tab_clone = tab.clone();
                                rsx! {
                                    button {
                                        key: "{tab.label()}",
                                        class: format!(
                                            "flex-1 flex items-center justify-center gap-2 px-4 py-2 rounded-lg text-xs font-bold transition-all {}",
                                            if is_active { "bg-[#3B82F6] text-white shadow-sm" }
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
                                supervisor_id: supervisor.id,
                                supervisor_agents,
                                agents,
                            }
                        },
                    }

                    // ── Save footer ────────────────────────────────────
                    div { class: "px-8 py-6 bg-white border border-[#E2E8F0] rounded-2xl shadow-soft flex justify-between items-center",
                        div { class: "flex items-center gap-2 text-slate-400",
                            MaterialIcon {
                                name: "verified",
                                size: 18,
                                color: MaterialIconColor::Custom("#10b981".into())
                            }
                            span { class: "text-[10px] font-bold uppercase tracking-widest", "Runtime Ready" }
                        }
                        button {
                            class: "bg-slate-900 text-white px-8 py-3 rounded-xl text-xs font-bold hover:bg-slate-800 transition-all shadow-md active:scale-95 flex items-center gap-2",
                            onclick: {
                                let sv = supervisor.clone();
                                move |_| {
                                    let mut sv = sv.clone();
                                    sv.instructions = Some(local_instructions());
                                    sv.code = Some(local_code());
                                    let state = state;
                                    spawn(async move {
                                        let db = state.read().db.clone();
                                        let _ = update_supervisor(&db, sv).await;
                                    });
                                }
                            },
                            MaterialIcon { name: "save", size: 16 }
                            " Save & Deploy Supervisor "
                        }
                    }
                }

            } else {
                // ── Empty state ────────────────────────────────────────
                div { class: "h-full flex flex-col items-center justify-center text-center space-y-6",
                    div { class: "w-24 h-24 rounded-full bg-white border border-[#E2E8F0] shadow-soft flex items-center justify-center text-5xl",
                        "🛸"
                    }
                    div {
                        h2 { class: "text-2xl font-bold text-[#0F172A]", "Supervisor Command" }
                        p { class: "text-slate-400 mt-2 max-w-xs",
                            "Select a supervisor to edit its preamble, code logic, and agent assignments."
                        }
                    }
                }
            }
        }
    }
}
