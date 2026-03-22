use crate::db::Planner;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn PlannerGrid(
    planners: Resource<Vec<Planner>>,
    selected_planner: Signal<Option<Planner>>,
    search_query: Signal<String>,
    on_edit: EventHandler<Planner>,
    on_delete: EventHandler<i64>,
) -> Element {
    let list = planners.value()();

    rsx! {
        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-8 pb-20 animate-in fade-in slide-in-from-bottom-4 duration-700",
            match list {
                Some(items) => {
                    let filtered: Vec<_> = items.iter()
                        .filter(|p| {
                            let q = search_query().to_lowercase();
                            p.name.to_lowercase().contains(&q) ||
                            p.domain.to_lowercase().contains(&q) ||
                            p.description.as_ref().map(|d| d.to_lowercase().contains(&q)).unwrap_or(false)
                        })
                        .cloned()
                        .collect();

                    if filtered.is_empty() {
                        rsx! {
                            div { class: "col-span-full py-20 flex flex-col items-center justify-center text-center space-y-4 bg-white/50 border-2 border-dashed border-slate-200 rounded-[2.5rem]",
                                div { class: "w-16 h-16 bg-slate-100 rounded-2xl flex items-center justify-center text-slate-400",
                                    MaterialIcon { name: "search_off", size: 32 }
                                }
                                div {
                                    h3 { class: "text-lg font-bold text-slate-900", "No planners found" }
                                    p { class: "text-sm text-slate-400", "Try adjusting your search or create a new planner." }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            for planner in filtered {
                                PlannerCard {
                                    key: "{planner.id}",
                                    planner,
                                    selected_planner,
                                    on_edit,
                                    on_delete,
                                }
                            }
                        }
                    }
                },
                None => rsx! {
                    for _ in 0..4 {
                        div { class: "h-[340px] bg-white border-2 border-slate-50 rounded-[2.5rem] animate-pulse shadow-sm" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn PlannerCard(
    planner: Planner,
    selected_planner: Signal<Option<Planner>>,
    on_edit: EventHandler<Planner>,
    on_delete: EventHandler<i64>,
) -> Element {
    let p_card = planner.clone();
    let p_edit = planner.clone();
    let p_del = planner.clone();

    rsx! {
        div {
            class: "group bg-white border-2 border-slate-200 rounded-[2.5rem] p-8 shadow-[0_8px_30px_rgb(0,0,0,0.04)] hover:shadow-[0_20px_50px_rgba(99,102,241,0.08)] hover:border-blue-100 hover:-translate-y-2 transition-all duration-500 cursor-pointer flex flex-col h-[340px] relative overflow-hidden",
            onclick: move |_| {
                selected_planner.set(Some(p_card.clone()));
            },

            // Subtle background glow
            div { class: "absolute -top-24 -right-24 w-48 h-48 bg-brand-blue/5 rounded-full blur-3xl group-hover:bg-slate-800/10 transition-colors duration-500" }

            // Card Header: Icon + Domain
            div { class: "flex items-start justify-between mb-6",
                div { class: "w-14 h-14 bg-slate-50 text-slate-400 rounded-2xl flex items-center justify-center group-hover:bg-brand-blue group-hover:text-white group-hover:shadow-xl group-hover:shadow-blue-200 transition-all duration-500",
                    MaterialIcon { name: "architecture", size: 28 }
                }
                div { class: "flex flex-col items-end gap-1",
                    span { class: "px-3 py-1 bg-slate-100 text-slate-600 text-[9px] font-black uppercase tracking-widest rounded-lg border border-slate-200 transition-colors group-hover:bg-blue-50 group-hover:text-brand-blue group-hover:border-blue-200",
                        "{planner.domain}"
                    }
                    span { class: "px-2 py-0.5 bg-brand-blue/10 text-brand-blue text-[8px] font-black uppercase tracking-widest rounded", "v{planner.version.clone().unwrap_or_else(|| \"1.0.0\".into())}" }
                }
            }

            // Content
            h3 { class: "text-xl font-black text-slate-900 mb-3 truncate group-hover:text-brand-blue transition-colors duration-300", "{planner.name}" }
            p { class: "text-slate-500 text-sm font-medium line-clamp-3 leading-relaxed mb-8 flex-1",
                {planner.description.as_deref().unwrap_or("No strategic objective defined for this planner node.")}
            }

            // Footer
            div { class: "mt-auto pt-6 border-t border-slate-50 flex items-center justify-between",
                div { class: "flex items-center gap-2",
                    span { class: "w-2 h-2 rounded-full bg-brand-blue group-hover:animate-ping" }
                    span { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "{planner.model.as_deref().unwrap_or(\"gpt-4o\")}" }
                }
                div { class: "flex gap-2 opacity-0 group-hover:opacity-100 transition-all duration-300 translate-y-2 group-hover:translate-y-0",
                    button {
                        class: "w-10 h-10 flex items-center justify-center bg-slate-50 text-slate-400 hover:bg-brand-blue hover:text-white rounded-xl transition-all shadow-sm active:scale-90",
                        title: "Edit Strategy",
                        onclick: move |e| {
                            e.stop_propagation();
                            on_edit.call(p_edit.clone());
                        },
                        MaterialIcon { name: "edit", size: 18 }
                    }
                    button {
                        class: "w-10 h-10 flex items-center justify-center bg-slate-50 text-slate-400 hover:bg-red-600 hover:text-white rounded-xl transition-all shadow-sm active:scale-90",
                        title: "Decommission",
                        onclick: move |e| {
                            e.stop_propagation();
                            on_delete.call(p_del.id);
                        },
                        MaterialIcon { name: "delete_sweep", size: 18 }
                    }
                }
            }
        }
    }
}
