use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::{Planner, Database};
use crate::backend::{get_planners, add_planner, update_planner, delete_planner};
use crate::state::use_global_state;

pub mod list;
pub mod details;
pub mod modal;
pub mod hero_card;

pub use list::PlannerGrid;
pub use details::PlannerDetails;
pub use modal::PlannerModal;

#[component]
pub fn PlannersWorkshop() -> Element {
    let state = use_global_state();
    
    let planners: Resource<Vec<Planner>> = use_resource(move || {
        let db = state.read().db.clone();
        async move {
            get_planners(&db).await.unwrap_or_default()
        }
    });

    let selected_planner = use_signal(|| None::<Planner>);
    let mut show_modal = use_signal(|| false);
    let mut editing_planner = use_signal(|| None::<Planner>);
    
    // ── Editing Signals ──────────────────────────────────────────────────
    let mut name_input = use_signal(|| String::new());
    let mut description_input = use_signal(|| String::new());
    let mut domain_input = use_signal(|| String::new());
    let mut instructions_input = use_signal(|| String::new());
    let mut code_input = use_signal(|| String::new());
    let mut search_query = use_signal(|| String::new());

    let handle_save = move |()| {
        let state = state;
        let mut planners = planners;
        spawn(async move {
            let db = state.read().db.clone();
            if let Some(mut p) = editing_planner() {
                p.name = name_input();
                p.description = Some(description_input());
                p.domain = domain_input();
                p.instructions = Some(instructions_input());
                p.code = Some(code_input());
                let _ = update_planner(&db, p).await;
            } else {
                let _ = add_planner(&db, &name_input(), &description_input(), &domain_input()).await;
            }
            show_modal.set(false);
            planners.restart();
        });
    };

    let handle_delete = move |id: i64| {
        let state = state;
        let mut planners = planners;
        spawn(async move {
            let db = state.read().db.clone();
            let _ = delete_planner(&db, id).await;
            planners.restart();
        });
    };

    let mut on_edit = move |p: Planner| {
        editing_planner.set(Some(p.clone()));
        name_input.set(p.name.clone());
        description_input.set(p.description.clone().unwrap_or_default());
        domain_input.set(p.domain.clone());
        instructions_input.set(p.instructions.clone().unwrap_or_default());
        code_input.set(p.code.clone().unwrap_or_default());
        show_modal.set(true);
    };

    rsx! {
        div { class: "flex-1 flex flex-col h-screen bg-slate-50 overflow-hidden",
            
            // ── Unified Workshop Header ──────────────────────────────────────
            div { class: "flex flex-col shrink-0 px-10 pt-10 pb-6 space-y-8 animate-in fade-in slide-in-from-top-4 duration-700",
                div { class: "flex items-center justify-between",
                    div { class: "flex items-center gap-5",
                        div { class: "w-12 h-12 bg-white rounded-2xl flex items-center justify-center border border-slate-200 shadow-soft",
                            MaterialIcon { name: "architecture", size: 24, color: MaterialIconColor::Custom("var(--color-brand-blue)".into()) }
                        }
                        div {
                            h1 { class: "text-2xl font-black text-slate-900 tracking-tight", "Planners Workshop" }
                            p { class: "text-slate-400 text-xs font-bold uppercase tracking-widest mt-1", "Domain Orchestration Service" }
                        }
                    }

                    div { class: "flex items-center gap-4",
                        // Search Bar
                        div { class: "relative group",
                            div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-slate-300 group-focus-within:text-brand-blue transition-colors flex items-center justify-center",
                                MaterialIcon { 
                                    name: "search", 
                                    size: 18, 
                                }
                            }
                            input {
                                class: "w-64 bg-white border border-slate-200 rounded-xl pl-11 pr-4 py-2.5 text-sm focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all placeholder-slate-300 shadow-soft",
                                placeholder: "Filter planners...",
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value()),
                            }
                        }
                        
                        // Create Button
                        button { 
                            class: "bg-brand-blue text-white px-6 py-2.5 rounded-xl text-xs font-bold uppercase tracking-widest hover:bg-blue-500 transition-all shadow-lg shadow-blue-200 active:scale-95 flex items-center gap-2",
                            onclick: move |_| {
                                editing_planner.set(None);
                                name_input.set(String::new());
                                description_input.set(String::new());
                                domain_input.set(String::new());
                                instructions_input.set(String::new());
                                code_input.set(String::new());
                                show_modal.set(true);
                            },
                            MaterialIcon { name: "add", size: 16 }
                            "Initialize Planner"
                        }
                    }
                }
            }

            // ── View Surface ───────────────────────────────────────────────
            div { class: "flex-1 overflow-y-auto px-10 pb-10 custom-scrollbar",
                if selected_planner().is_none() {
                    // Grid View
                    PlannerGrid {
                        planners,
                        selected_planner,
                        search_query,
                        on_edit: move |p| on_edit(p),
                        on_delete: move |id| handle_delete(id),
                    }
                } else {
                    // Detail View
                    PlannerDetails {
                        selected_planner,
                        planners,
                    }
                }
            }

            // ── Modal ─────────────────────────────────────────────────────
            if show_modal() {
                PlannerModal { 
                    show_modal, 
                    editing_planner, 
                    name_input, 
                    description_input,
                    domain_input, 
                    instructions_input,
                    code_input,
                    handle_save 
                }
            }
        }
    }
}
