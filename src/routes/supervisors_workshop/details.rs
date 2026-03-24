use crate::backend::{
    get_agents, get_available_models, get_supervisor_agents, get_supervisor_tools,
    update_supervisor,
};
use crate::db::Supervisor;
use crate::engine::utils::discovery::AgentInfo;
use crate::state::use_global_state;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

use super::agent_assignments::AgentAssignments;
use super::code_editor::CodeEditor;
use super::hero_card::SupervisorHeroCard;
use super::preamble_editor::PreambleEditor;

#[derive(Clone, PartialEq)]
enum Tab {
    Preamble,
    Code,
    Agents,
}

impl Tab {
    fn label(&self) -> &'static str {
        match self {
            Tab::Preamble => "Execution Preamble",
            Tab::Code => "Orchestration Logic",
            Tab::Agents => "Agent Assignments",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Tab::Preamble => "description",
            Tab::Code => "code",
            Tab::Agents => "hub",
        }
    }
}

#[component]
pub fn SupervisorDetails(
    mut selected_supervisor: Signal<Option<Supervisor>>,
    mut supervisors: Resource<Vec<Supervisor>>,
) -> Element {
    let state = use_global_state();

    let supervisor = selected_supervisor();
    let supervisor_id = supervisor.as_ref().map(|s| s.id).unwrap_or(0);

    let mut supervisor_agents = use_resource(move || async move {
        get_supervisor_agents(&state.read().db, supervisor_id)
            .await
            .unwrap_or_default()
    });

    let mut supervisor_tools = use_resource(move || async move {
        get_supervisor_tools(&state.read().db, supervisor_id)
            .await
            .unwrap_or_default()
    });

    let agents = use_resource(move || async move {
        get_agents(&state.read().db).await.unwrap_or_default()
    });

    let mut models = use_signal(|| vec!["loading...".to_string()]);
    let _ = use_resource(move || async move {
        let fetched = get_available_models().await;
        if !fetched.is_empty() {
            models.set(fetched);
        }
    });

    let mut local_instructions = use_signal(|| {
        supervisor
            .as_ref()
            .and_then(|sv| sv.instructions.clone())
            .unwrap_or_default()
    });
    let mut local_code = use_signal(|| {
        supervisor
            .as_ref()
            .and_then(|sv| sv.code.clone())
            .unwrap_or_default()
    });
    let mut save_feedback = use_signal(|| "Supervisor configuration is synced.".to_string());
    let mut save_state = use_signal(|| "synced".to_string());

    use_effect(move || {
        if let Some(sv) = selected_supervisor() {
            local_instructions.set(sv.instructions.clone().unwrap_or_default());
            local_code.set(sv.code.clone().unwrap_or_default());
            supervisor_agents.restart();
            supervisor_tools.restart();
            save_feedback.set("Supervisor configuration is synced.".to_string());
            save_state.set("synced".to_string());
        }
    });

    let mut active_tab = use_signal(|| Tab::Preamble);

    rsx! {
        div { class: "w-full bg-slate-50 overflow-y-auto p-8 custom-scrollbar h-full",
            if let Some(sv_final) = selected_supervisor() {
                {
                    let assigned_agents_count = supervisor_agents.read().as_ref().map(|items| items.len()).unwrap_or(0);
                    let tool_count = supervisor_tools.read().as_ref().map(|items| items.len()).unwrap_or(0);
                    let is_dirty = sv_final.instructions.clone().unwrap_or_default() != local_instructions()
                        || sv_final.code.clone().unwrap_or_default() != local_code();
                    let display_state = if is_dirty && save_state() == "synced" {
                        "draft".to_string()
                    } else {
                        save_state()
                    };
                    let display_feedback = if is_dirty && save_state() == "synced" {
                        "Draft changes are local until you deploy them.".to_string()
                    } else {
                        save_feedback()
                    };

                    rsx! {
                        div { class: "max-w-6xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500",
                            SupervisorHeroCard {
                                selected_supervisor,
                                models,
                                agents: Signal::new(agents.read().as_ref().cloned().unwrap_or_default()),
                                assigned_agents_count,
                                tool_count,
                                workbench_status: display_feedback.clone(),
                                save_state: display_state.clone(),
                            }

                            div { class: "grid grid-cols-1 md:grid-cols-4 gap-4",
                                DetailMetricCard {
                                    label: "Assigned Agents".to_string(),
                                    value: assigned_agents_count.to_string(),
                                    tone: "blue".to_string(),
                                    caption: "Operators currently linked to this supervisor.".to_string(),
                                }
                                DetailMetricCard {
                                    label: "Reachable Tools".to_string(),
                                    value: tool_count.to_string(),
                                    tone: "emerald".to_string(),
                                    caption: "Available through linked agents.".to_string(),
                                }
                                DetailMetricCard {
                                    label: "Output Path".to_string(),
                                    value: if sv_final.bypass_synthesizer { "Direct".to_string() } else { "Synthesized".to_string() },
                                    tone: "amber".to_string(),
                                    caption: "Controls how final supervisor output is emitted.".to_string(),
                                }
                                DetailMetricCard {
                                    label: "Runtime Model".to_string(),
                                    value: sv_final.model.clone().unwrap_or_else(|| "Default".to_string()),
                                    tone: "slate".to_string(),
                                    caption: "Model currently assigned to the execution lane.".to_string(),
                                }
                            }

                            div { class: "grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_280px] gap-6 items-start",
                                div { class: "space-y-8",
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
                                }

                                div { class: "space-y-4 xl:sticky xl:top-6",
                                    SidePanelCard {
                                        eyebrow: "Delegation Readout".to_string(),
                                        title: format!("`{}` is this supervisor's execution lane.", sv_final.domain),
                                        body: vec![
                                            "Requests in this domain should reach this supervisor before tools fire.".to_string(),
                                            "Linked agents decide what tools and domain behavior are reachable at runtime.".to_string(),
                                            "Use direct output only when you want this supervisor to bypass the final synthesis layer.".to_string(),
                                        ],
                                    }
                                    SidePanelCard {
                                        eyebrow: "Workbench Status".to_string(),
                                        title: display_state.replace('_', " ").to_uppercase(),
                                        body: vec![
                                            display_feedback.clone(),
                                            format!("Model: {}", sv_final.model.clone().unwrap_or_else(|| "Default runtime".to_string())),
                                        ],
                                    }
                                }
                            }

                            div { class: "px-8 py-6 bg-white border border-slate-200 rounded-2xl shadow-soft flex justify-between items-center",
                                div { class: "flex items-center gap-2 text-slate-400",
                                    MaterialIcon {
                                        name: "verified",
                                        size: 18,
                                        color: MaterialIconColor::Custom("var(--color-brand-blue)".into())
                                    }
                                    span { class: "text-[10px] font-bold uppercase tracking-widest", "Delegation control plane active" }
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
                                                save_state.set("saving".to_string());
                                                save_feedback.set("Deploying supervisor changes...".to_string());
                                                let state = state;
                                                let mut supervisors = supervisors.clone();
                                                let mut supervisor_agents = supervisor_agents.clone();
                                                let mut supervisor_tools = supervisor_tools.clone();
                                                spawn(async move {
                                                    let db = state.read().db.clone();
                                                    let outcome = update_supervisor(&db, sv).await;
                                                    if outcome.is_ok() {
                                                        save_feedback.set("Supervisor configuration deployed successfully.".to_string());
                                                        save_state.set("synced".to_string());
                                                    } else {
                                                        save_feedback.set("Failed to deploy supervisor changes.".to_string());
                                                        save_state.set("error".to_string());
                                                    }
                                                    supervisors.restart();
                                                    supervisor_agents.restart();
                                                    supervisor_tools.restart();
                                                });
                                            }
                                        },
                                        MaterialIcon { name: "save", size: 16 }
                                        "Save & Deploy Node"
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "h-full flex flex-col items-center justify-center text-center space-y-6",
                    div { class: "w-24 h-24 rounded-full bg-white border border-slate-200 shadow-soft flex items-center justify-center text-5xl", "S" }
                    div {
                        h2 { class: "text-2xl font-bold text-slate-900", "Supervisor Command" }
                        p { class: "text-slate-400 mt-2 max-w-xs", "Select a supervisor node to manage its orchestration logic and agent fleet." }
                    }
                }
            }
        }
    }
}

#[component]
fn DetailMetricCard(label: String, value: String, tone: String, caption: String) -> Element {
    let tone_class = match tone.as_str() {
        "blue" => "bg-blue-50 border-blue-100 text-blue-700",
        "emerald" => "bg-emerald-50 border-emerald-100 text-emerald-700",
        "amber" => "bg-amber-50 border-amber-100 text-amber-700",
        _ => "bg-slate-50 border-slate-200 text-slate-700",
    };

    rsx! {
        div { class: format!("rounded-2xl border p-4 shadow-soft {}", tone_class),
            p { class: "text-[10px] font-black uppercase tracking-[0.18em] opacity-70", "{label}" }
            p { class: "mt-3 text-lg font-black break-words", "{value}" }
            p { class: "mt-2 text-xs opacity-80 leading-relaxed", "{caption}" }
        }
    }
}

#[component]
fn SidePanelCard(eyebrow: String, title: String, body: Vec<String>) -> Element {
    rsx! {
        div { class: "bg-white border border-slate-200 rounded-2xl shadow-soft p-5 space-y-4",
            div {
                p { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "{eyebrow}" }
                h3 { class: "text-lg font-black text-slate-900 mt-1 leading-snug", "{title}" }
            }
            ul { class: "space-y-3 text-sm text-slate-600 leading-relaxed",
                for line in body {
                    li { "{line}" }
                }
            }
        }
    }
}
