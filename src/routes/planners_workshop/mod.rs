use dioxus::prelude::*;
use crate::db::{Planner, Database};
use crate::backend::{get_planners, add_planner, update_planner, delete_planner};
use crate::state::use_global_state;

pub mod list;
pub mod details;
pub mod modal;

pub use list::PlannerList;
pub use details::PlannerDetails;
pub use modal::PlannerModal;

#[component]
pub fn PlannersWorkshop() -> Element {
    let state = use_global_state();
    
    let mut planners: Resource<Vec<Planner>> = use_resource(move || {
        let db = state.read().db.clone();
        async move {
            get_planners(&db).await.unwrap_or_default()
        }
    });

    let mut selected_planner = use_signal(|| None::<Planner>);
    let mut show_modal = use_signal(|| false);
    let mut editing_planner = use_signal(|| None::<Planner>);
    let mut name_input = use_signal(|| String::new());
    let mut description_input = use_signal(|| String::new());
    let mut domain_input = use_signal(|| String::new());

    let handle_save = move |()| {
        let state = state;
        let mut planners = planners;
        spawn(async move {
            let db = state.read().db.clone();
            if let Some(mut p) = editing_planner() {
                p.name = name_input();
                p.description = Some(description_input());
                p.domain = domain_input();
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

    rsx! {
        div { class: "flex h-screen bg-[#F8FAFC] overflow-hidden",
            // Sidebar List
            PlannerList { 
                planners, 
                selected_planner, 
                editing_planner, 
                name_input, 
                description_input,
                domain_input, 
                show_modal, 
                handle_delete 
            }

            // Main Editor Surface
            div { class: "flex-1 flex flex-col min-w-0 bg-[#F8FAFC]",
                // Toolbar
                div { class: "h-16 border-b border-[#E2E8F0] bg-white flex items-center justify-between px-8 shrink-0",
                    div { class: "flex items-center gap-3",
                        div { class: "w-8 h-8 rounded-lg bg-indigo-50 flex items-center justify-center text-indigo-600 font-bold", "P" }
                        h1 { class: "text-lg font-bold text-[#0F172A]", "Planners Workshop" }
                    }
                    button { 
                        class: "bg-slate-900 text-white px-4 py-2 rounded-xl text-xs font-bold hover:bg-slate-800 transition-all shadow-md active:scale-95 flex items-center gap-2",
                        onclick: move |_| {
                            editing_planner.set(None);
                            name_input.set(String::new());
                            description_input.set(String::new());
                            domain_input.set(String::new());
                            show_modal.set(true);
                        },
                        "＋ Create Planner"
                    }
                }

                // Details Area
                PlannerDetails { selected_planner }
            }

            // Modals
            if show_modal() {
                PlannerModal { 
                    show_modal, 
                    editing_planner, 
                    name_input, 
                    description_input,
                    domain_input, 
                    handle_save 
                }
            }
        }
    }
}
