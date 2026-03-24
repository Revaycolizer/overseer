use crate::backend::{
    add_supervisor, delete_supervisor, get_available_models, get_supervisors, update_supervisor,
};
use crate::db::Supervisor;
use crate::state::use_global_state;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

pub mod agent_assignments;
pub mod code_editor;
pub mod details;
pub mod hero_card;
pub mod list;
pub mod modal;
pub mod preamble_editor;
pub mod test_panel;

pub use details::SupervisorDetails;
pub use list::SupervisorGrid;
pub use modal::SupervisorModal;

#[component]
pub fn SupervisorsWorkshop() -> Element {
    let state = use_global_state();

    let mut supervisors: Resource<Vec<Supervisor>> = use_resource(move || {
        let db = state.read().db.clone();
        async move {
            let res: Vec<Supervisor> = get_supervisors(&db).await.unwrap_or_default();
            res
        }
    });

    let mut selected_supervisor = use_signal(|| None::<Supervisor>);
    let mut show_modal = use_signal(|| false);
    let mut editing_supervisor = use_signal(|| None::<Supervisor>);

    // ── Input State ──────────────────────────────────────────────────────
    let mut name_input = use_signal(|| String::new());
    let mut description_input = use_signal(|| String::new());
    let mut domain_input = use_signal(|| String::new());
    let mut instructions_input = use_signal(|| String::new());
    let mut model_input = use_signal(|| String::new());
    let mut code_input = use_signal(|| String::new());
    let mut bypass_synthesizer_input = use_signal(|| false);
    let mut search_query = use_signal(|| String::new());
    let mut fast_path_keywords_input = use_signal(|| String::new());
    let mut verification_agent_id_input = use_signal(|| None::<i64>);
    let mut routing_priority_input = use_signal(|| 0i32);
    let mut hide_evidence_input = use_signal(|| false);
    let mut report_style_input = use_signal(|| "detailed".to_string());

    // ── Pre-fetch models ─────────────────────────────────────────────────
    let mut available_models = use_signal(|| vec!["loading...".to_string()]);
    let _ = use_resource(move || async move {
        let fetched: Vec<String> = get_available_models().await;
        if !fetched.is_empty() {
            available_models.set(fetched);
        }
    });

    let handle_save = move |()| {
        let state = state;
        let mut supervisors = supervisors;
        spawn(async move {
            let db = state.read().db.clone();
            if let Some(mut s) = editing_supervisor() {
                s.name = name_input();
                s.description = Some(description_input());
                s.domain = domain_input();
                s.instructions = Some(instructions_input());
                s.model = Some(model_input());
                s.code = Some(code_input());
                s.bypass_synthesizer = bypass_synthesizer_input();
                s.fast_path_keywords = Some(fast_path_keywords_input());
                s.verification_agent_id = verification_agent_id_input();
                s.routing_priority = routing_priority_input();
                s.hide_evidence = hide_evidence_input();
                s.report_style = Some(report_style_input());
                let _ = update_supervisor(&db, s).await;
            } else {
                let _ =
                    add_supervisor(&db, &name_input(), &description_input(), &domain_input()).await;
            }
            show_modal.set(false);
            supervisors.restart();
        });
    };

    let handle_delete = move |id: i64| {
        let state = state;
        let mut supervisors = supervisors;
        spawn(async move {
            let db = state.read().db.clone();
            let _ = delete_supervisor(&db, id).await;
            supervisors.restart();
        });
    };

    let mut on_edit = move |s: Supervisor| {
        editing_supervisor.set(Some(s.clone()));
        name_input.set(s.name.clone());
        description_input.set(s.description.clone().unwrap_or_default());
        domain_input.set(s.domain.clone());
        instructions_input.set(s.instructions.clone().unwrap_or_default());
        model_input.set(s.model.clone().unwrap_or_else(|| "gpt-4o".into()));
        code_input.set(s.code.clone().unwrap_or_default());
        bypass_synthesizer_input.set(s.bypass_synthesizer);
        fast_path_keywords_input.set(s.fast_path_keywords.clone().unwrap_or_default());
        verification_agent_id_input.set(s.verification_agent_id);
        routing_priority_input.set(s.routing_priority);
        hide_evidence_input.set(s.hide_evidence);
        report_style_input.set(s.report_style.clone().unwrap_or_else(|| "detailed".to_string()));
        show_modal.set(true);
    };

    rsx! {
        div { class: "flex-1 flex flex-col h-screen bg-slate-50 overflow-hidden",

            // ── Unified Workshop Header ──────────────────────────────────────
            div { class: "flex flex-col shrink-0 px-10 pt-10 pb-6 space-y-8 animate-in fade-in slide-in-from-top-4 duration-700",
                div { class: "flex items-center justify-between",
                    div { class: "flex items-center gap-5",
                        div { class: "w-12 h-12 bg-white rounded-2xl flex items-center justify-center border border-slate-200 shadow-soft",
                            MaterialIcon { name: "hub", size: 24, color: MaterialIconColor::Custom("var(--color-brand-blue)".into()) }
                        }
                        div {
                            h1 { class: "text-2xl font-black text-slate-900 tracking-tight", "Supervisors Workshop" }
                            p { class: "text-slate-400 text-[10px] font-bold uppercase tracking-widest mt-1", "Fleet Orchestration Service" }
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
                                placeholder: "Filter nodes...",
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value()),
                            }
                        }

                        // Create Button
                        button {
                            class: "bg-brand-blue text-white px-6 py-2.5 rounded-xl text-xs font-bold uppercase tracking-widest hover:bg-blue-500 transition-all shadow-lg shadow-blue-100 active:scale-95 flex items-center gap-2",
                            onclick: move |_| {
                                editing_supervisor.set(None);
                                name_input.set(String::new());
                                description_input.set(String::new());
                                domain_input.set(String::new());
                                instructions_input.set(String::new());
                                model_input.set("gpt-4o".into());
                                code_input.set(String::new());
                                bypass_synthesizer_input.set(false);
                                fast_path_keywords_input.set(String::new());
                                verification_agent_id_input.set(None);
                                routing_priority_input.set(0);
                                hide_evidence_input.set(false);
                                report_style_input.set("detailed".to_string());
                                show_modal.set(true);
                            },
                            MaterialIcon { name: "add", size: 16 }
                            "Deploy New Node"
                        }
                    }
                }
            }

            // ── View Surface ───────────────────────────────────────────────
            div { class: "flex-1 overflow-y-auto px-10 pb-10 custom-scrollbar",
                if selected_supervisor().is_none() {
                    // Grid View
                    SupervisorGrid {
                        supervisors,
                        selected_supervisor,
                        search_query,
                        on_edit: move |s: Supervisor| on_edit(s),
                        on_delete: move |id: i64| handle_delete(id),
                    }
                } else {
                    // Detail View (Flattened)
                    SupervisorDetails {
                        selected_supervisor,
                        supervisors,
                    }
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
                    fast_path_keywords: fast_path_keywords_input,
                    verification_agent_id: verification_agent_id_input,
                    routing_priority: routing_priority_input,
                    hide_evidence: hide_evidence_input,
                    report_style: report_style_input,
                    available_models,
                    handle_save
                }
            }
        }
    }
}
