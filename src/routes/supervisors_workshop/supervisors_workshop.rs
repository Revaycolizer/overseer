use dioxus::prelude::*;
use crate::backend::{
    get_supervisors, add_supervisor, update_supervisor, delete_supervisor,
    get_supervisor_agents, get_agents, get_available_models,
};
use crate::db::Supervisor;
use crate::engine::utils::discovery::AgentInfo;

use super::list::SupervisorList;
use super::details::SupervisorDetails;
use super::modal::SupervisorModal;

use crate::state::use_global_state;

#[component]
pub fn SupervisorsWorkshop() -> Element {
    let state = use_global_state();

    // ── Supervisor list ────────────────────────────────────────────────────
    let mut supervisors = use_resource(move || {
        let state = state;
        async move {
            let db = state.read().db.clone();
            get_supervisors(&db).await.unwrap_or_default()
        }
    });

    // ── Available models (shared with modal + details) ─────────────────────
    let mut available_models = use_signal(|| vec!["loading...".to_string()]);
    let _ = use_resource(move || async move {
        let fetched: Vec<String> = get_available_models().await;
        if !fetched.is_empty() {
            available_models.set(fetched);
        }
    });

    // ── Modal form signals ────────────────────────────────────────────────
    let mut show_modal          = use_signal(|| false);
    let mut editing_supervisor  = use_signal(|| None::<Supervisor>);
    // basic
    let mut name_input          = use_signal(|| String::new());
    let mut description_input   = use_signal(|| String::new());
    let mut domain_input        = use_signal(|| String::new());
    // extended
    let mut instructions_input  = use_signal(|| String::new());
    let mut model_input         = use_signal(|| String::new());
    let mut code_input          = use_signal(|| String::new());
    let mut bypass_synthesizer_input = use_signal(|| false);

    // ── Selected supervisor + its agents ──────────────────────────────────
    let mut selected_supervisor = use_signal(|| None::<Supervisor>);
    let mut supervisor_agents   = use_resource(move || {
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

    // ── Handlers ──────────────────────────────────────────────────────────
    let mut handle_save = move |_| {
        let state = state;
        spawn(async move {
            let db = state.read().db.clone();
            if let Some(mut s) = editing_supervisor() {
                // Patch all fields
                s.name         = name_input();
                s.description  = Some(description_input());
                s.domain       = domain_input();
                s.instructions = Some(instructions_input());
                s.model        = Some(model_input());
                s.code         = Some(code_input());
                s.bypass_synthesizer = bypass_synthesizer_input();
                let _ = update_supervisor(&db, s).await;
            } else {
                let new_id = add_supervisor(
                    &db,
                    &name_input(),
                    &description_input(),
                    &domain_input(),
                ).await;
                // Persist extended fields on the freshly-created row
                if let Ok(id) = new_id {
                    let _ = update_supervisor(&db, crate::db::Supervisor {
                        id,
                        name: name_input(),
                        description: Some(description_input()),
                        domain: domain_input(),
                        instructions: Some(instructions_input()),
                        model: Some(model_input()),
                        code: Some(code_input()),
                        bypass_synthesizer: bypass_synthesizer_input(),
                        version: None,
                    }).await;
                }
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

    // ── Helpers to open modal in create / edit mode ───────────────────────
    let open_create = move |_| {
        editing_supervisor.set(None);
        name_input.set(String::new());
        description_input.set(String::new());
        domain_input.set(String::new());
        instructions_input.set(String::new());
        model_input.set(
            available_models().into_iter().next().unwrap_or_default()
        );
        code_input.set(String::new());
        bypass_synthesizer_input.set(false);
        show_modal.set(true);
    };

    // ─────────────────────────────────────────────────────────────────────
    rsx! {
        div { class: "flex flex-col h-screen bg-[#F8FAFC] text-[#0F172A] font-sans",

            // ── Header ────────────────────────────────────────────────────
            div { class: "px-6 py-4 border-b border-[#E2E8F0] flex justify-between items-center bg-white shadow-soft sticky top-0 z-10",
                div {
                    h1 { class: "text-xl font-bold tracking-tight text-[#0F172A]", "Supervisors Workshop" }
                    p { class: "text-slate-400 text-xs mt-0.5", "Manage orchestration hierarchies and agent assignments." }
                }
                button {
                    class: "px-4 py-2 bg-[#3B82F6] hover:bg-blue-500 text-white rounded-xl text-sm font-semibold transition-all duration-200 flex items-center gap-2 shadow-sm active:scale-95",
                    onclick: open_create,
                    span { class: "text-base leading-none", "＋" }
                    "New Supervisor"
                }
            }

            // ── Main content ──────────────────────────────────────────────
            div { class: "flex-1 flex flex-col md:flex-row overflow-hidden",
                SupervisorList {
                    supervisors,
                    selected_supervisor,
                    supervisor_agents,
                    editing_supervisor,
                    name_input,
                    description_input,
                    domain_input,
                    instructions_input,
                    model_input,
                    code_input,
                    bypass_synthesizer_input,
                    show_modal,
                    handle_delete,
                }
                SupervisorDetails {
                    selected_supervisor,
                    supervisor_agents,
                    agents,
                }
            }

            // ── Modal ─────────────────────────────────────────────────────
            if show_modal() {
                SupervisorModal {
                    show_modal,
                    editing_supervisor,
                    name_input,
                    description_input,
                    domain_input,
                    instructions_input,
                    model_input,
                    code_input,
                    bypass_synthesizer: bypass_synthesizer_input,
                    available_models,
                    handle_save,
                }
            }
        }
    }
}