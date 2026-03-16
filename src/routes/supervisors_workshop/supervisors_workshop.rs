use dioxus::prelude::*;
use crate::backend::{get_supervisors, add_supervisor, update_supervisor, delete_supervisor, get_supervisor_agents, get_agents, link_agent_to_supervisor, unlink_agent_from_supervisor};
use crate::db::{Supervisor, Database};
use crate::engine::utils::discovery::AgentInfo;

use super::list::SupervisorList;
use super::details::SupervisorDetails;
use super::modal::SupervisorModal;

use crate::state::use_global_state;

#[component]
pub fn SupervisorsWorkshop() -> Element {
    let state = use_global_state();
    
    let mut supervisors = use_resource(move || {
        let state = state;
        async move { 
            let db = state.read().db.clone();
            get_supervisors(&db).await.unwrap_or_default() 
        }
    });

    let mut show_modal = use_signal(|| false);
    let mut editing_supervisor = use_signal(|| None::<Supervisor>);
    let mut name_input = use_signal(|| String::new());
    let mut description_input = use_signal(|| String::new());
    let mut domain_input = use_signal(|| String::new());

    let mut selected_supervisor = use_signal(|| None::<Supervisor>);
    let mut supervisor_agents = use_resource(move || {
        let state = state;
        let s = selected_supervisor();
        async move {
            if let Some(supervisor) = s {
                let db = state.read().db.clone();
                get_supervisor_agents(&db, supervisor.id).await.unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let agents = use_resource(move || {
        let state = state;
        async move { 
            let db = state.read().db.clone();
            get_agents(&db).await.unwrap_or_default() 
        }
    });

    let mut handle_save = move |_| {
        let state = state;
        spawn(async move {
            let db = state.read().db.clone();
            if let Some(mut s) = editing_supervisor() {
                s.name = name_input();
                s.description = Some(description_input());
                s.domain = domain_input();
                let _ = update_supervisor(&db, s).await;
            } else {
                let _ = add_supervisor(&db, &name_input(), &description_input(), &domain_input()).await;
            }
            show_modal.set(false);
            supervisors.restart();
        });
    };

    let mut handle_delete = move |id: i64| {
        let state = state;
        spawn(async move {
            let db = state.read().db.clone();
            let _ = delete_supervisor(&db, id).await;
            supervisors.restart();
            if selected_supervisor().map(|s| s.id == id).unwrap_or(false) {
                selected_supervisor.set(None);
            }
        });
    };

    rsx! {
        div { class: "flex flex-col h-screen bg-[#F8FAFC] text-[#0F172A] font-sans",
            // Header
            div { class: "p-6 border-b border-[#E2E8F0] flex justify-between items-center bg-white shadow-soft sticky top-0 z-10",
                div {
                    h1 { class: "text-2xl font-bold tracking-tight text-[#0F172A]", "Supervisors Workshop" }
                    p { class: "text-slate-400 text-sm mt-1", "Manage dynamic orchestration hierarchies and agent assignments." }
                }
                button { 
                    class: "px-4 py-2 bg-[#3B82F6] hover:bg-blue-500 text-white rounded-lg transition-all duration-200 flex items-center gap-2 shadow-sm",
                    onclick: move |_| {
                        editing_supervisor.set(None);
                        name_input.set(String::new());
                        description_input.set(String::new());
                        domain_input.set(String::new());
                        show_modal.set(true);
                    },
                    span { class: "text-lg", "＋" }
                    "New Supervisor"
                }
            }

            // Main Content
            div { class: "flex-1 flex flex-col md:flex-row overflow-hidden",
                SupervisorList { 
                    supervisors, 
                    selected_supervisor, 
                    supervisor_agents,
                    editing_supervisor,
                    name_input,
                    description_input,
                    domain_input,
                    show_modal,
                    handle_delete
                }
                SupervisorDetails { 
                    selected_supervisor, 
                    supervisor_agents, 
                    agents 
                }
            }

            if show_modal() {
                SupervisorModal { 
                    show_modal, 
                    editing_supervisor, 
                    name_input, 
                    description_input,
                    domain_input, 
                    handle_save 
                }
            }
        }
    }
}