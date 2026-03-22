use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::Supervisor;
use crate::engine::utils::discovery::AgentInfo;

#[component]
pub fn SupervisorList(
    mut supervisors: Resource<Vec<Supervisor>>,
    mut selected_supervisor: Signal<Option<Supervisor>>,
    mut supervisor_agents: Resource<Vec<AgentInfo>>,
    mut editing_supervisor: Signal<Option<Supervisor>>,
    mut name_input: Signal<String>,
    mut description_input: Signal<String>,
    mut domain_input: Signal<String>,
    mut instructions_input: Signal<String>,
    mut model_input: Signal<String>,
    mut code_input: Signal<String>,
    mut bypass_synthesizer_input: Signal<bool>,
    mut show_modal: Signal<bool>,
    handle_delete: EventHandler<i64>,
    search_query: Signal<String>,
) -> Element {
    rsx! {
        div { class: "w-full md:w-80 flex flex-shrink-0 flex-col border-r border-slate-200 bg-white overflow-hidden",

            // ── List Header ─────────────────────────────────────────────
            div { class: "px-4 py-3 border-b border-slate-100 flex items-center justify-between bg-slate-50/30",
                span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "All Supervisors" }
                match supervisors.value()() {
                    Some(list) => {
                        rsx! {
                            span { class: "text-[10px] bg-slate-200 text-slate-600 px-2 py-0.5 rounded-full font-bold",
                                "{list.len()}"
                            }
                        }
                    },
                    None => rsx! { div { class: "w-4 h-4 rounded bg-slate-100 animate-pulse" } }
                }
            }

            // ── List ─────────────────────────────────────────────────────
            div { class: "flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar",
                match supervisors.value()() {
                    Some(list) => {
                        let query = search_query().to_lowercase();
                        let filtered: Vec<_> = list.into_iter()
                            .filter(|s| {
                                query.is_empty()
                                    || s.name.to_lowercase().contains(&query)
                                    || s.domain.to_lowercase().contains(&query)
                                    || s.description.as_deref().unwrap_or("").to_lowercase().contains(&query)
                            })
                            .collect();

                        if filtered.is_empty() {
                            rsx! {
                                div { class: "flex flex-col items-center justify-center py-12 text-center",
                                    p { class: "text-xs font-semibold text-slate-400", "No partial matches" }
                                }
                            }
                        } else {
                            rsx! {
                                for s in filtered {
                                    {
                                        let s_click = s.clone();
                                        let s_edit  = s.clone();
                                        let s_del   = s.clone();
                                        let is_selected = selected_supervisor()
                                            .map(|sel| sel.id == s.id)
                                            .unwrap_or(false);

                                        rsx! {
                                            div {
                                                key: "{s.id}",
                                                class: format!(
                                                    "group relative flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition-all duration-150 {}",
                                                    if is_selected {
                                                        "bg-blue-50 border-blue-200 shadow-sm"
                                                    } else {
                                                        "bg-white border-transparent hover:bg-slate-50"
                                                    }
                                                ),
                                                onclick: move |_| {
                                                    selected_supervisor.set(Some(s_click.clone()));
                                                    supervisor_agents.restart();
                                                },

                                                // ── Icon ─────────────────────
                                                div { class: format!(
                                                    "mt-0.5 w-7 h-7 rounded-lg flex items-center justify-center shrink-0 {}",
                                                    if is_selected { "bg-blue-600 text-white" } else { "bg-slate-100 text-slate-400" }
                                                ),
                                                    span { class: "text-[10px] font-bold", "S" }
                                                }

                                                // ── Text block ────────────────────────────
                                                div { class: "flex-1 min-w-0",
                                                    h3 { class: format!(
                                                        "text-xs font-bold truncate {}",
                                                        if is_selected { "text-blue-900" } else { "text-slate-900" }
                                                    ),
                                                        "{s.name}"
                                                    }
                                                    p { class: "text-[10px] text-slate-400 truncate mt-0.5", "{s.domain}" }
                                                }

                                                // ── Actions ─────────────────
                                                div { class: "opacity-0 group-hover:opacity-100 transition-opacity flex gap-0.5",
                                                    button {
                                                        class: "p-1 hover:bg-white rounded text-slate-400 hover:text-blue-600 transition-colors",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            editing_supervisor.set(Some(s_edit.clone()));
                                                            name_input.set(s_edit.name.clone());
                                                            description_input.set(s_edit.description.clone().unwrap_or_default());
                                                            domain_input.set(s_edit.domain.clone());
                                                            instructions_input.set(s_edit.instructions.clone().unwrap_or_default());
                                                            model_input.set(s_edit.model.clone().unwrap_or_default());
                                                            code_input.set(s_edit.code.clone().unwrap_or_default());
                                                            bypass_synthesizer_input.set(s_edit.bypass_synthesizer);
                                                                    show_modal.set(true);
                                                        },
                                                        span { class: "text-[10px]", "✎" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! { div { "Loading..." } }
                }
            }

            // ── Footer ─────────────────────────────────────────────
            div { class: "px-4 py-3 border-t border-slate-100 bg-slate-50/50",
                match supervisors.value()() {
                    Some(list) => {
                        let query = search_query();
                        let shown = if query.is_empty() { list.len() } else {
                            list.iter().filter(|s| {
                                let q = query.to_lowercase();
                                s.name.to_lowercase().contains(&q)
                                    || s.domain.to_lowercase().contains(&q)
                                    || s.description.as_deref().unwrap_or("").to_lowercase().contains(&q)
                            }).count()
                        };
                        rsx! {
                            p { class: "text-[10px] text-slate-400 font-bold uppercase tracking-tighter", "{shown} Supervisors Found" }
                        }
                    },
                    None => rsx! { p { "..." } }
                }
            }
        }
    }
}