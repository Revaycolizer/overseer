use dioxus::prelude::*;
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
    mut show_modal: Signal<bool>,
    handle_delete: EventHandler<i64>,
) -> Element {
    rsx! {
        div { class: "w-full md:w-1/3 border-r border-[#E2E8F0] overflow-y-auto p-4 custom-scrollbar bg-white",
            div { class: "space-y-3",
                match supervisors.value()() {
                    Some(list) => {
                        rsx! {
                            for s in list {
                                {
                                    let s_click = s.clone();
                                    let s_edit = s.clone();
                                    let s_del = s.clone();
                                    rsx! {
                                        div { 
                                            key: "{s.id}",
                                            class: format!("p-4 rounded-xl border transition-all duration-200 cursor-pointer group {}", 
                                                if selected_supervisor().map(|sel| sel.id == s.id).unwrap_or(false) 
                                                { "bg-[#EFF6FF] border-[#3B82F6]/40 ring-1 ring-[#3B82F6]/20" } 
                                                else { "bg-white border-[#E2E8F0] hover:border-slate-300" }
                                            ),
                                            onclick: move |_| {
                                                selected_supervisor.set(Some(s_click.clone()));
                                                supervisor_agents.restart();
                                            },
                                            div { class: "flex justify-between items-start",
                                                div {
                                                    h3 { class: "font-semibold text-[#0F172A]", "{s.name}" }
                                                    span { class: "text-xs px-2 py-0.5 bg-slate-100 text-slate-500 rounded-full mt-1 inline-block border border-[#E2E8F0]", "{s.domain}" }
                                                }
                                                div { class: "flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity",
                                                    button { 
                                                        class: "p-1.5 hover:bg-slate-100 rounded-md text-slate-400 hover:text-slate-700 transition-colors",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            editing_supervisor.set(Some(s_edit.clone()));
                                                            name_input.set(s_edit.name.clone());
                                                            description_input.set(s_edit.description.clone().unwrap_or_default());
                                                            domain_input.set(s_edit.domain.clone());
                                                            show_modal.set(true);
                                                        },
                                                        "✎"
                                                    }
                                                    button { 
                                                        class: "p-1.5 hover:bg-red-50 rounded-md text-slate-400 hover:text-red-500 transition-colors",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            handle_delete.call(s_del.id);
                                                        },
                                                        "✕"
                                                    }
                                                }
                                            }
                                            p { class: "text-xs text-slate-400 mt-2 line-clamp-2", "{s.description.as_deref().unwrap_or_default()}" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! { div { class: "p-8 text-center text-slate-400 animate-pulse", "Loading supervisors..." } }
                }
            }
        }
    }
}