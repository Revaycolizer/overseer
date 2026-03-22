use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use crate::backend::{
    get_supervisors, add_supervisor, update_supervisor, delete_supervisor,
    get_supervisor_agents, get_agents, get_available_models,
};
use crate::db::Supervisor;

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
    let mut search_query = use_signal(|| String::new());

    // ── Selected supervisor + its agents ──────────────────────────────────
    let mut selected_supervisor = use_signal(|| None::<Supervisor>);
    let supervisor_agents   = use_resource(move || {
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
    let handle_save = move |_| {
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

    let handle_delete = move |id| {
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
    let mut open_create = move || {
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
        div { class: "flex flex-col h-screen bg-slate-50 text-slate-900 font-sans",
            
            // ── Main Scrollable Content ───────────────────────────────────
            div { class: "flex-1 overflow-y-auto custom-scrollbar p-10 bg-slate-50/50",
                div { class: "max-w-7xl mx-auto space-y-12",
                    
                    // 1. BRAND & NAVIGATION HEADER (Scrollable)
                    div { class: "flex flex-col md:flex-row md:items-center justify-between gap-6",
                        div { class: "flex items-center gap-4",
                            div { class: "flex flex-col",
                                h1 { class: "text-2xl font-extrabold tracking-tight text-slate-900 flex items-center gap-2",
                                    span { class: "text-blue-600", "✦" }
                                    "Supervisors Workshop"
                                }
                                p { class: "text-slate-500 text-[11px] font-medium uppercase tracking-wider mt-0.5", "Orchestration & Hierarchy Engine" }
                            }
                            div { class: "w-px h-8 bg-slate-200 mx-2 hidden md:block" }
                            div { class: "flex items-center gap-2 px-3 py-1.5 bg-emerald-50 rounded-full border border-emerald-100",
                                span { class: "w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" }
                                span { class: "text-[9px] font-black text-emerald-600 uppercase tracking-widest", "Live" }
                            }
                        }
                        
                        if selected_supervisor().is_some() {
                            button {
                                class: "group px-6 py-2.5 bg-slate-100 hover:bg-slate-200 text-slate-600 hover:text-slate-900 text-sm font-black rounded-2xl flex items-center gap-2.5 transition-all active:scale-95 shadow-sm self-start md:self-auto",
                                onclick: move |_| selected_supervisor.set(None),
                                MaterialIcon { name: "arrow_back", size: 18 }
                                "Back"
                            }
                        }
                    }

                    // 2. UNIFIED ACTION ROW (Search + Add)
                    div { class: "flex flex-col md:flex-row items-center gap-6 pb-6 border-b border-slate-100",
                        // Global Search
                        div { class: "flex-1 w-full relative group",
                            div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-slate-400 group-focus-within:text-blue-500 transition-colors",
                                span { class: "text-lg", "⌕" }
                            }
                            input {
                                class: "w-full bg-slate-100/50 border border-slate-200 rounded-2xl pl-11 pr-4 py-3 text-sm focus:bg-white focus:outline-none focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500/40 transition-all placeholder-slate-400 font-medium",
                                placeholder: "Search supervisors by name, domain or model...",
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value()),
                            }
                        }
                        
                        // Initialize Button
                        button {
                            class: "w-full md:w-auto bg-blue-600 text-white px-8 py-3.5 rounded-2xl text-sm font-black hover:bg-blue-700 transition-all shadow-xl shadow-blue-500/20 active:scale-95 flex items-center justify-center gap-2 shrink-0",
                            onclick: move |_| open_create(),
                            MaterialIcon { name: "add_circle", size: 20 }
                            "Initialize Supervisor"
                        }
                    }

                    // 3. MAIN DYNAMIC CONTENT
                    {
                        if selected_supervisor().is_none() {
                            // Grid Mode
                            match supervisors.value()() {
                                Some(list) => {
                                    let query = search_query().to_lowercase();
                                    let filtered: Vec<_> = list.into_iter()
                                        .filter(|s| {
                                            let q = query.clone();
                                            q.is_empty()
                                                || s.name.to_lowercase().contains(&q)
                                                || s.domain.to_lowercase().contains(&q)
                                                || s.description.as_deref().unwrap_or("").to_lowercase().contains(&q)
                                        })
                                        .collect();

                                    if filtered.is_empty() {
                                        rsx! {
                                            div { class: "flex flex-col items-center justify-center py-32 animate-in fade-in zoom-in-95 duration-700",
                                                div { class: "w-24 h-24 bg-white border-4 border-slate-100 rounded-[2.5rem] flex items-center justify-center mb-8 text-slate-200 shadow-xl",
                                                    MaterialIcon { name: "search_off", size: 48 }
                                                }
                                                h3 { class: "text-2xl font-black text-slate-900 tracking-tight", "No Nodes Detected" }
                                                p { class: "text-slate-500 mt-3 font-medium text-center max-w-xs", "We couldn't find any supervisors matching your search parameters." }
                                                button {
                                                    class: "mt-8 px-8 py-3 bg-slate-100 hover:bg-slate-200 text-slate-600 font-black text-xs uppercase tracking-widest rounded-2xl transition-all active:scale-95",
                                                    onclick: move |_| search_query.set(String::new()),
                                                    "Reset Search Filter"
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-8 pb-20 animate-in fade-in slide-in-from-bottom-6 duration-700",
                                                for s in filtered {
                                                    SupervisorCard {
                                                        key: "{s.id}",
                                                        supervisor: s.clone(),
                                                        selected_supervisor,
                                                        on_delete: move |id| handle_delete(id),
                                                        on_edit: move |s_edit: Supervisor| {
                                                            name_input.set(s_edit.name.clone());
                                                            description_input.set(s_edit.description.clone().unwrap_or_default());
                                                            domain_input.set(s_edit.domain.clone());
                                                            instructions_input.set(s_edit.instructions.clone().unwrap_or_default());
                                                            model_input.set(s_edit.model.clone().unwrap_or_default());
                                                            code_input.set(s_edit.code.clone().unwrap_or_default());
                                                            bypass_synthesizer_input.set(s_edit.bypass_synthesizer);
                                                            editing_supervisor.set(Some(s_edit.clone()));
                                                            show_modal.set(true);
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                None => rsx! {
                                    div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-8",
                                        for _ in 0..8 {
                                            div { class: "bg-white border border-slate-100 rounded-[2.5rem] p-8 h-[340px] animate-pulse shadow-sm flex flex-col",
                                                div { class: "flex justify-between mb-6",
                                                    div { class: "w-14 h-14 bg-slate-50 rounded-2xl" }
                                                    div { class: "w-24 h-6 bg-slate-50 rounded-full" }
                                                }
                                                div { class: "h-8 bg-slate-100 rounded-lg w-3/4 mb-4" }
                                                div { class: "flex-1 space-y-3",
                                                    div { class: "h-3 bg-slate-50 rounded-full w-full" }
                                                    div { class: "h-3 bg-slate-50 rounded-full w-5/6" }
                                                    div { class: "h-3 bg-slate-50 rounded-full w-2/3" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Detail Mode: Sidebar + Details
                            rsx! {
                                div { class: "flex flex-col lg:flex-row gap-10 items-start pb-20",
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
                                        search_query,
                                    }
                                    SupervisorDetails {
                                        selected_supervisor,
                                        supervisors,
                                        supervisor_agents,
                                        agents,
                                    }
                                }
                            }
                        }
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
                    available_models,
                    agents,
                    handle_save,
                }
            }
        }
    }
}

#[component]
fn SupervisorCard(
    supervisor: Supervisor,
    selected_supervisor: Signal<Option<Supervisor>>,
    on_delete: EventHandler<i64>,
    on_edit: EventHandler<Supervisor>,
) -> Element {
    let s_card = supervisor.clone();
    let s_edit_card = supervisor.clone();
    let s_del_card = supervisor.clone();

    rsx! {
        div {
            class: "group bg-white border-2 border-slate-100 rounded-[2.5rem] p-8 shadow-[0_8px_30px_rgb(0,0,0,0.04)] hover:shadow-[0_20px_50px_rgba(59,130,246,0.08)] hover:border-blue-100 hover:-translate-y-2 transition-all duration-500 cursor-pointer flex flex-col h-[340px] relative overflow-hidden",
            onclick: move |_| {
                selected_supervisor.set(Some(s_card.clone()));
            },
            
            // Subtle background glow
            div { class: "absolute -top-24 -right-24 w-48 h-48 bg-blue-500/5 rounded-full blur-3xl group-hover:bg-blue-500/10 transition-colors duration-500" }

            // Card Header: Icon + Domain
            div { class: "flex items-start justify-between mb-6",
                div { class: "w-14 h-14 bg-slate-50 text-slate-400 rounded-2xl flex items-center justify-center group-hover:bg-blue-600 group-hover:text-white group-hover:shadow-xl group-hover:shadow-blue-500/30 transition-all duration-500",
                    MaterialIcon { name: "hub", size: 28 }
                }
                div { class: "flex flex-col items-end gap-1",
                    span { class: "px-3 py-1 bg-slate-100 text-slate-600 text-[9px] font-black uppercase tracking-widest rounded-lg border border-slate-200 transition-colors group-hover:bg-blue-50 group-hover:text-blue-600 group-hover:border-blue-200",
                        "{supervisor.domain}"
                    }
                    if supervisor.bypass_synthesizer {
                        span { class: "px-2 py-0.5 bg-amber-500/10 text-amber-600 text-[8px] font-black uppercase tracking-widest rounded", "Direct" }
                    }
                }
            }

            // Content
            h3 { class: "text-xl font-black text-slate-900 mb-3 truncate group-hover:text-blue-600 transition-colors duration-300", "{supervisor.name}" }
            p { class: "text-slate-500 text-sm font-medium line-clamp-3 leading-relaxed mb-8 flex-1", 
                {supervisor.description.as_deref().unwrap_or("No operational summary provided for this node.")}
            }

            // Footer
            div { class: "mt-auto pt-6 border-t border-slate-50 flex items-center justify-between",
                div { class: "flex items-center gap-2",
                    span { class: "w-2 h-2 rounded-full bg-blue-500 group-hover:animate-ping" }
                    span { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "{supervisor.model.as_deref().unwrap_or(\"llama3\")}" }
                }
                div { class: "flex gap-2 opacity-0 group-hover:opacity-100 transition-all duration-300 translate-y-2 group-hover:translate-y-0",
                    button { 
                        class: "w-10 h-10 flex items-center justify-center bg-slate-50 text-slate-400 hover:bg-blue-600 hover:text-white rounded-xl transition-all shadow-sm active:scale-90",
                        title: "Edit Infrastructure",
                        onclick: move |e| {
                            e.stop_propagation();
                            on_edit.call(s_edit_card.clone());
                        },
                        MaterialIcon { name: "edit", size: 18 }
                    }
                    button { 
                        class: "w-10 h-10 flex items-center justify-center bg-slate-50 text-slate-400 hover:bg-red-600 hover:text-white rounded-xl transition-all shadow-sm active:scale-90",
                        title: "Terminate Node",
                        onclick: move |e| {
                            e.stop_propagation();
                            on_delete.call(s_del_card.id);
                        },
                        MaterialIcon { name: "delete_sweep", size: 18 }
                    }
                }
            }
        }
    }
}