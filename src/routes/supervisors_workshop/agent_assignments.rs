use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::backend::{link_agent_to_supervisor, unlink_agent_from_supervisor};
use crate::engine::utils::discovery::AgentInfo;
use crate::state::use_global_state;

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
                    class: "text-sm font-semibold text-slate-400 uppercase tracking-wider",
                    "Managed Agents"
                }
                p { class: "text-xs text-slate-400", "Select agents this supervisor can dispatch." }
            }

            // Agent grid
            match (agents.value()(), supervisor_agents.value()()) {
                (Some(all), Some(assigned)) => rsx! {
                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                        for agent in all {
                            {
                                let s_id = supervisor_id;
                                let is_assigned = assigned.iter().any(|a| a.id == agent.id);
                                rsx! {
                                    div {
                                        key: "{agent.id}",
                                        class: format!(
                                            "p-5 rounded-2xl border transition-all duration-200 flex items-center justify-between {}",
                                            if is_assigned {
                                                "bg-blue-50 border-[#3B82F6]/30 shadow-sm"
                                            } else {
                                                "bg-white border-[#E2E8F0] hover:border-slate-300"
                                            }
                                        ),

                                        // Agent info
                                        div { class: "flex items-center gap-3",
                                            div {
                                                class: format!(
                                                    "w-8 h-8 rounded-lg flex items-center justify-center {}",
                                                    if is_assigned { "bg-white text-[#3B82F6]" } else { "bg-slate-50 text-slate-400" }
                                                ),
                                                MaterialIcon {
                                                    name: if is_assigned { "check_circle" } else { "radio_button_unchecked" },
                                                    size: 20
                                                }
                                            }
                                            div {
                                                h4 {
                                                    class: format!(
                                                        "text-sm font-bold {}",
                                                        if is_assigned { "text-[#1E40AF]" } else { "text-[#0F172A]" }
                                                    ),
                                                    "{agent.name}"
                                                }
                                                p {
                                                    class: "text-[10px] text-slate-400 font-mono uppercase tracking-tighter",
                                                    "{agent.domain}"
                                                }
                                            }
                                        }

                                        // Toggle button
                                        button {
                                            class: format!(
                                                "p-2 rounded-lg transition-all {}",
                                                if is_assigned {
                                                    "text-[#3B82F6] hover:bg-[#3B82F6]/10"
                                                } else {
                                                    "text-slate-400 hover:text-slate-600 hover:bg-slate-100"
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
                                                name: if is_assigned { "remove_circle_outline" } else { "add_circle_outline" },
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
                    div { class: "p-8 text-center text-slate-300 animate-pulse text-sm", "Loading agents..." }
                }
            }
        }
    }
}
