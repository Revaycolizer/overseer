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
) -> Element {
    // Local search filter
    let mut search = use_signal(|| String::new());

    rsx! {
        div { class: "w-full md:w-1/3 flex flex-col border-r border-[#E2E8F0] bg-white overflow-hidden",

            // ── Search bar ───────────────────────────────────────────────
            div { class: "px-4 pt-4 pb-3 border-b border-[#E2E8F0]",
                div { class: "relative",
                    div { class: "absolute left-3 top-1/2 -translate-y-1/2 text-slate-300 pointer-events-none",
                        MaterialIcon { name: "search", size: 16 }
                    }
                    input {
                        class: "w-full bg-[#F8FAFC] border border-[#E2E8F0] rounded-xl pl-9 pr-4 py-2.5 text-sm text-[#0F172A] placeholder-slate-300 focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6]/40 transition-all",
                        placeholder: "Search supervisors...",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                    }
                }
            }

            // ── List ─────────────────────────────────────────────────────
            div { class: "flex-1 overflow-y-auto p-3 space-y-1.5 custom-scrollbar",
                match supervisors.value()() {
                    Some(list) => {
                        let query = search().to_lowercase();
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
                                div { class: "flex flex-col items-center justify-center py-16 text-center",
                                    div { class: "w-12 h-12 rounded-2xl bg-slate-50 border border-[#E2E8F0] flex items-center justify-center mb-3",
                                        MaterialIcon { name: "search_off", size: 22, color: MaterialIconColor::Custom("#cbd5e1".into()) }
                                    }
                                    p { class: "text-sm font-semibold text-slate-400", "No supervisors found" }
                                    p { class: "text-xs text-slate-300 mt-1", "Try a different search term" }
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
                                                    "group relative flex items-start gap-3 p-3.5 rounded-xl border cursor-pointer transition-all duration-150 {}",
                                                    if is_selected {
                                                        "bg-[#EFF6FF] border-[#3B82F6]/30 shadow-soft ring-1 ring-[#3B82F6]/10"
                                                    } else {
                                                        "bg-white border-[#E2E8F0] hover:border-slate-300 hover:bg-slate-50/60"
                                                    }
                                                ),
                                                onclick: move |_| {
                                                    selected_supervisor.set(Some(s_click.clone()));
                                                    supervisor_agents.restart();
                                                },

                                                // ── Domain colour dot ─────────────────────
                                                div { class: format!(
                                                    "mt-0.5 w-8 h-8 rounded-xl flex items-center justify-center shrink-0 {}",
                                                    if is_selected {
                                                        "bg-[#3B82F6]/10 text-[#3B82F6]"
                                                    } else {
                                                        "bg-slate-100 text-slate-400"
                                                    }
                                                ),
                                                    MaterialIcon {
                                                        name: "account_tree",
                                                        size: 16,
                                                        color: if is_selected {
                                                            MaterialIconColor::Custom("#3b82f6".into())
                                                        } else {
                                                            MaterialIconColor::Custom("#94a3b8".into())
                                                        }
                                                    }
                                                }

                                                // ── Text block ────────────────────────────
                                                div { class: "flex-1 min-w-0",
                                                    div { class: "flex items-center justify-between gap-2",
                                                        h3 { class: format!(
                                                            "text-sm font-semibold truncate {}",
                                                            if is_selected { "text-[#1E40AF]" } else { "text-[#0F172A]" }
                                                        ),
                                                            "{s.name}"
                                                        }
                                                        // Action buttons — fade in on hover
                                                        div { class: "flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0",
                                                            button {
                                                                class: "p-1.5 rounded-lg hover:bg-white text-slate-400 hover:text-[#3B82F6] hover:shadow-soft transition-all",
                                                                title: "Edit",
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
                                                                MaterialIcon { name: "edit", size: 13 }
                                                            }
                                                            button {
                                                                class: "p-1.5 rounded-lg hover:bg-red-50 text-slate-400 hover:text-red-500 hover:shadow-soft transition-all",
                                                                title: "Delete",
                                                                onclick: move |e| {
                                                                    e.stop_propagation();
                                                                    handle_delete.call(s_del.id);
                                                                },
                                                                MaterialIcon { name: "delete_outline", size: 13 }
                                                            }
                                                        }
                                                    }

                                                    // Domain pill + model badge
                                                    div { class: "flex items-center gap-1.5 mt-1 flex-wrap",
                                                        span { class: format!(
                                                            "inline-flex items-center gap-1 text-[10px] font-bold px-2 py-0.5 rounded-full uppercase tracking-wider {}",
                                                            if is_selected {
                                                                "bg-[#3B82F6]/10 text-[#3B82F6]"
                                                            } else {
                                                                "bg-slate-100 text-slate-500"
                                                            }
                                                        ),
                                                            MaterialIcon { name: "hub", size: 10 }
                                                            "{s.domain}"
                                                        }
                                                        if let Some(model) = s.model.as_ref() {
                                                            if !model.is_empty() {
                                                                span { class: "inline-flex items-center text-[10px] font-mono px-2 py-0.5 rounded-full bg-slate-50 border border-[#E2E8F0] text-slate-400 truncate max-w-[110px]",
                                                                    "{model}"
                                                                }
                                                            }
                                                        }
                                                    }

                                                    // Description
                                                    if let Some(desc) = s.description.as_deref() {
                                                        if !desc.is_empty() {
                                                            p { class: "text-xs text-slate-400 mt-1.5 line-clamp-2 leading-relaxed",
                                                                "{desc}"
                                                            }
                                                        }
                                                    }
                                                }

                                                // ── Selected indicator bar ─────────────────
                                                if is_selected {
                                                    div { class: "absolute left-0 top-3 bottom-3 w-0.5 bg-[#3B82F6] rounded-r-full" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! {
                        // Skeleton loading
                        div { class: "space-y-2 p-1",
                            for _ in 0..4 {
                                div { class: "p-3.5 rounded-xl border border-[#E2E8F0] animate-pulse",
                                    div { class: "flex items-start gap-3",
                                        div { class: "w-8 h-8 rounded-xl bg-slate-100 shrink-0" }
                                        div { class: "flex-1 space-y-2",
                                            div { class: "h-3.5 bg-slate-100 rounded-full w-3/5" }
                                            div { class: "h-2.5 bg-slate-50 rounded-full w-1/3" }
                                            div { class: "h-2.5 bg-slate-50 rounded-full w-4/5 mt-1" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Footer count ─────────────────────────────────────────────
            div { class: "px-4 py-3 border-t border-[#E2E8F0] bg-slate-50/60",
                match supervisors.value()() {
                    Some(list) => {
                        let total = list.len();
                        let query = search();
                        let shown = if query.is_empty() { total } else {
                            list.iter().filter(|s| {
                                let q = query.to_lowercase();
                                s.name.to_lowercase().contains(&q)
                                    || s.domain.to_lowercase().contains(&q)
                                    || s.description.as_deref().unwrap_or("").to_lowercase().contains(&q)
                            }).count()
                        };
                        let count_label = if query.is_empty() {
                            let s = if total == 1 { "" } else { "s" };
                            format!("{total} supervisor{s}")
                        } else {
                            format!("{shown} of {total} matched")
                        };
                        rsx! {
                            p { class: "text-[10px] text-slate-400 font-medium", "{count_label}" }
                        }
                    },
                    None => rsx! {
                        p { class: "text-[10px] text-slate-300 animate-pulse", "Loading..." }
                    }
                }
            }
        }
    }
}