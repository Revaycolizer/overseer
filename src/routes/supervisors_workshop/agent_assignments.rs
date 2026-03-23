use crate::backend::{link_agent_to_supervisor, unlink_agent_from_supervisor};
use crate::engine::utils::discovery::AgentInfo;
use crate::state::use_global_state;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn AgentAssignments(
    supervisor_id: i64,
    mut supervisor_agents: Resource<Vec<AgentInfo>>,
    agents: Resource<Vec<AgentInfo>>,
) -> Element {
    let state = use_global_state();

    rsx! {
        div { class: "space-y-4",
            // Section header
            div { class: "flex justify-between items-center",
                h3 {
                    class: "text-sm font-black text-slate-400 uppercase tracking-widest",
                    "Fleet Assignments"
                }
                p { class: "text-[10px] font-bold text-slate-400 uppercase tracking-tighter bg-slate-100 px-2 py-0.5 rounded", "Managed Node Cluster" }
            }

            // Agent grid
            match (agents.value()(), supervisor_agents.value()()) {
                (Some(all), Some(assigned)) => rsx! {
                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-3",
                        AssignmentMetric {
                            label: "Assigned".to_string(),
                            value: assigned.len().to_string(),
                            tone: "blue".to_string(),
                        }
                        AssignmentMetric {
                            label: "Available".to_string(),
                            value: all.len().to_string(),
                            tone: "slate".to_string(),
                        }
                        AssignmentMetric {
                            label: "Unassigned".to_string(),
                            value: (all.len().saturating_sub(assigned.len())).to_string(),
                            tone: "emerald".to_string(),
                        }
                    }
                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                        for agent in all {
                            {
                                let s_id = supervisor_id;
                                let is_assigned = assigned.iter().any(|a| a.id == agent.id);
                                rsx! {
                                    div {
                                        key: "{agent.id}",
                                        class: format!(
                                            "p-5 rounded-2xl border transition-all duration-300 flex items-center justify-between group {}",
                                            if is_assigned {
                                                "bg-blue-50 border-blue-200 shadow-soft"
                                            } else {
                                                "bg-white border-slate-200 hover:border-slate-300"
                                            }
                                        ),

                                        // Agent info
                                        div { class: "flex items-center gap-4",
                                            div {
                                                class: format!(
                                                    "w-10 h-10 rounded-xl flex items-center justify-center transition-all duration-300 {}",
                                                    if is_assigned { "bg-white text-brand-blue shadow-sm" } else { "bg-slate-50 text-slate-400" }
                                                ),
                                                MaterialIcon {
                                                    name: if is_assigned { "verified" } else { "circle" },
                                                    size: 20
                                                }
                                            }
                                            div {
                                                h4 {
                                                    class: format!(
                                                        "text-sm font-black tracking-tight {}",
                                                        if is_assigned { "text-blue-900" } else { "text-slate-900" }
                                                    ),
                                                    "{agent.name}"
                                                }
                                                p {
                                                    class: "text-[9px] text-slate-400 font-bold uppercase tracking-widest mt-0.5",
                                                    "{agent.domain}"
                                                }
                                            }
                                        }

                                        // Toggle button
                                        button {
                                            class: format!(
                                                "w-8 h-8 rounded-lg flex items-center justify-center transition-all {}",
                                                if is_assigned {
                                                    "text-brand-blue hover:bg-white hover:shadow-sm"
                                                } else {
                                                    "text-slate-300 hover:text-slate-600 hover:bg-slate-100"
                                                }
                                            ),
                                            onclick: move |e| {
                                                e.stop_propagation();
                                                let a_id = agent.id;
                                                let state = state;
                                                spawn(async move {
                                                    let db = state.read().db.clone();
                                                    if is_assigned {
                                                        let _ = unlink_agent_from_supervisor(&db, s_id, a_id).await;
                                                    } else {
                                                        let _ = link_agent_to_supervisor(&db, s_id, a_id).await;
                                                    }
                                                    supervisor_agents.restart();
                                                });
                                            },
                                            MaterialIcon {
                                                name: if is_assigned { "remove_circle" } else { "add_circle" },
                                                size: 20
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                _ => rsx! {
                    div { class: "p-8 text-center text-slate-300 animate-pulse text-sm", "Synchronizing fleet data..." }
                }
            }
        }
    }
}

#[component]
fn AssignmentMetric(label: String, value: String, tone: String) -> Element {
    let tone_class = match tone.as_str() {
        "blue" => "bg-blue-50 border-blue-100 text-blue-700",
        "emerald" => "bg-emerald-50 border-emerald-100 text-emerald-700",
        _ => "bg-slate-50 border-slate-200 text-slate-700",
    };

    rsx! {
        div { class: format!("rounded-2xl border p-4 {}", tone_class),
            p { class: "text-[10px] font-black uppercase tracking-widest opacity-70", "{label}" }
            p { class: "mt-2 text-lg font-black", "{value}" }
        }
    }
}
