use dioxus::prelude::*;
use crate::db::Planner;

#[component]
pub fn PlannerList(
    mut planners: Resource<Vec<Planner>>,
    mut selected_planner: Signal<Option<Planner>>,
    mut editing_planner: Signal<Option<Planner>>,
    mut name_input: Signal<String>,
    mut description_input: Signal<String>,
    mut domain_input: Signal<String>,
    mut show_modal: Signal<bool>,
    handle_delete: EventHandler<i64>,
) -> Element {
    rsx! {
        div { class: "w-full md:w-1/3 border-r border-[#E2E8F0] overflow-y-auto p-4 custom-scrollbar bg-white",
            div { class: "space-y-3",
                match planners.value()() {
                    Some(list) => {
                        rsx! {
                            for p in list {
                                {
                                    let p_click = p.clone();
                                    let p_edit = p.clone();
                                    let p_del = p.clone();
                                    rsx! {
                                        div { 
                                            key: "{p.id}",
                                            class: format!("p-4 rounded-xl border transition-all duration-200 cursor-pointer group {}", 
                                                if selected_planner().map(|sel| sel.id == p.id).unwrap_or(false) 
                                                { "bg-[#EFF6FF] border-[#3B82F6]/40 ring-1 ring-[#3B82F6]/20" } 
                                                else { "bg-white border-[#E2E8F0] hover:border-slate-300" }
                                            ),
                                            onclick: move |_| {
                                                selected_planner.set(Some(p_click.clone()));
                                            },
                                            div { class: "flex justify-between items-start",
                                                div {
                                                    h3 { class: "font-semibold text-[#0F172A]", "{p.name}" }
                                                    span { class: "text-xs px-2 py-0.5 bg-slate-100 text-slate-500 rounded-full mt-1 inline-block border border-[#E2E8F0]", "{p.domain}" }
                                                }
                                                div { class: "flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity",
                                                    button { 
                                                        class: "p-1.5 hover:bg-slate-100 rounded-md text-slate-400 hover:text-slate-700 transition-colors",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            editing_planner.set(Some(p_edit.clone()));
                                                            name_input.set(p_edit.name.clone());
                                                            description_input.set(p_edit.description.clone().unwrap_or_default());
                                                            domain_input.set(p_edit.domain.clone());
                                                            show_modal.set(true);
                                                        },
                                                        "✎"
                                                    }
                                                    button { 
                                                        class: "p-1.5 hover:bg-red-50 rounded-md text-slate-400 hover:text-red-500 transition-colors",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            handle_delete.call(p_del.id);
                                                        },
                                                        "✕"
                                                    }
                                                }
                                            }
                                            p { class: "text-xs text-slate-400 mt-2 line-clamp-2", "{p.description.as_deref().unwrap_or_default()}" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! { div { class: "p-8 text-center text-slate-400 animate-pulse", "Loading planners..." } }
                }
            }
        }
    }
}
