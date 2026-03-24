use super::agent_assignments::AgentAssignments;
use crate::backend::{get_agents, get_supervisor_agents};
use crate::db::Supervisor;
use crate::engine::utils::discovery::AgentInfo;
use crate::state::use_global_state;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[derive(Clone, Copy, PartialEq)]
enum ModalTab {
    Basic,
    Agents,
    Preamble,
    Code,
}

#[component]
pub fn SupervisorModal(
    show_modal: Signal<bool>,
    editing_supervisor: Signal<Option<Supervisor>>,
    name_input: Signal<String>,
    description_input: Signal<String>,
    domain_input: Signal<String>,
    instructions_input: Signal<String>,
    model_input: Signal<String>,
    code_input: Signal<String>,
    bypass_synthesizer: Signal<bool>,
    fast_path_keywords: Signal<String>,
    verification_agent_id: Signal<Option<i64>>,
    routing_priority: Signal<i32>,
    hide_evidence: Signal<bool>,
    report_style: Signal<String>,
    available_models: Signal<Vec<String>>,
    handle_save: EventHandler<()>,
) -> Element {
    let state = use_global_state();
    let is_editing = editing_supervisor().is_some();
    let mut active_tab = use_signal(|| ModalTab::Basic);
    // ── Resources for Agent Tab ──────────────────────────────────────────
    let modal_supervisor_agents = use_resource(move || {
        let db = state.read().db.clone();
        let sid = editing_supervisor().map(|s| s.id).unwrap_or(0);
        async move {
            let res: Vec<crate::engine::utils::discovery::AgentInfo> = if sid > 0 {
                get_supervisor_agents(&db, sid).await.unwrap_or_default()
            } else {
                vec![]
            };
            res
        }
    });

    let agents = use_resource(move || {
        let db = state.read().db.clone();
        async move {
            let res: Vec<crate::engine::utils::discovery::AgentInfo> =
                get_agents(&db).await.unwrap_or_default();
            res
        }
    });

    // Handle Prism highlighting when switching to Code tab
    const PRISM_HIGHLIGHT_JS: &str = "if (window.Prism) { window.Prism.highlightAll(); }";

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/40 backdrop-blur-sm animate-in fade-in duration-300",
            onclick: move |_| show_modal.set(false),

            // ── Dialog ────────────────────────────────────────────────────
            div {
                class: "w-full max-w-xl bg-white border border-slate-200 rounded-[2.5rem] shadow-2xl flex flex-col animate-in zoom-in-95 duration-300 overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // ── Header ─────────────────────────────────────────────
                div { class: "flex items-center justify-between px-10 pt-10 pb-6",
                    div {
                        div { class: "flex items-center gap-5",
                            div { class: "w-12 h-12 rounded-2xl bg-blue-50 text-brand-blue border border-blue-100 flex items-center justify-center shadow-sm",
                                MaterialIcon {
                                    name: "hub",
                                    size: 24,
                                    color: MaterialIconColor::Custom("inherit".into())
                                }
                            }
                            div {
                                h2 { class: "text-xl font-black text-slate-900 tracking-tight",
                                    if is_editing { "Edit Supervisor" } else { "Deploy Supervisor" }
                                }
                                p { class: "text-slate-400 text-[10px] font-bold uppercase tracking-widest mt-0.5",
                                    if is_editing {
                                        "Synchronizing node infrastructure"
                                    } else {
                                        "Initializing new orchestration stack"
                                    }
                                }
                            }
                        }
                    }
                    button {
                        class: "w-8 h-8 flex items-center justify-center rounded-xl bg-slate-50 text-slate-400 hover:bg-slate-100 hover:text-slate-600 transition-all",
                        onclick: move |_| show_modal.set(false),
                        MaterialIcon { name: "close", size: 18 }
                    }
                }

                // ── Tab bar ────────────────────────────────────────────
                div { class: "flex gap-1 mx-10 mb-2 bg-slate-50 border border-slate-200 rounded-xl p-1",
                    for (tab, label, icon) in [
                        (ModalTab::Basic,    "Identity",  "badge"),
                        (ModalTab::Agents,   "Fleet",     "hub"),
                        (ModalTab::Preamble, "Strategy",  "description"),
                        (ModalTab::Code,     "Logic",     "code"),
                    ] {
                        {
                            let is_active = active_tab() == tab;
                            let tab_c = tab.clone();
                            rsx! {
                                button {
                                    key: "{label}",
                                    class: format!(
                                        "flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-[10px] font-black transition-all {}",
                                        if is_active { "bg-brand-blue text-white shadow-md shadow-blue-100" }
                                        else { "text-slate-400 hover:text-slate-600" }
                                    ),
                                    onclick: move |_| {
                                        active_tab.set(tab_c.clone());
                                    },
                                    MaterialIcon {
                                        name: icon,
                                        size: 14,
                                        color: if is_active { MaterialIconColor::Light } else { MaterialIconColor::Custom("inherit".into()) }
                                    }
                                    span { class: "uppercase tracking-widest", "{label}" }
                                }
                            }
                        }
                    }
                }

                // ── Tab content ────────────────────────────────────────
                div { class: "flex-1 px-10 py-6 space-y-6 overflow-y-auto max-h-[420px] custom-scrollbar",

                    // ── IDENTITY tab ───────────────────────────────────
                    if active_tab() == ModalTab::Basic {
                        div { class: "space-y-6 animate-in fade-in duration-200",

                            // Name
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Node Identifier" }
                                input {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-900 focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all placeholder-slate-300 shadow-soft",
                                    placeholder: "e.g. Research Orchestrator",
                                    value: "{name_input}",
                                    oninput: move |e| name_input.set(e.value()),
                                }
                            }

                            // Description
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Operational Objective" }
                                input {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-medium text-slate-600 focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all placeholder-slate-300 shadow-soft",
                                    placeholder: "e.g. Coordinates web-based data extraction",
                                    value: "{description_input}",
                                    oninput: move |e| description_input.set(e.value()),
                                }
                            }

                            // Domain + Model side by side
                            div { class: "grid grid-cols-2 gap-6",
                                div { class: "space-y-2",
                                    label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Domain" }
                                    input {
                                        class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-600 focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all placeholder-slate-300 shadow-soft",
                                        placeholder: "e.g. web",
                                        value: "{domain_input}",
                                        oninput: move |e| domain_input.set(e.value()),
                                    }
                                }
                                div { class: "space-y-2",
                                    label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Model" }
                                    div { class: "relative",
                                        select {
                                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-600 appearance-none focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all cursor-pointer shadow-soft",
                                            value: "{model_input}",
                                            onchange: move |e| model_input.set(e.value()),
                                            for model in available_models() {
                                                option { value: "{model}", "{model}" }
                                            }
                                        }
                                        div { class: "absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400",
                                            MaterialIcon { name: "unfold_more", size: 16 }
                                        }
                                    }
                                }
                            }

                            // Fast-Path Keywords
                            div { class: "space-y-2",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Fast-Path Routing Keywords" }
                                input {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-medium text-slate-600 focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all placeholder-slate-300 shadow-soft",
                                    placeholder: "e.g. music, play, volume, pause (comma separated)",
                                    value: "{fast_path_keywords}",
                                    oninput: move |e| fast_path_keywords.set(e.value()),
                                }
                            }

                            // Verification Agent + Priority
                            div { class: "grid grid-cols-2 gap-6",
                                div { class: "space-y-2",
                                    label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Grounding / Verification Agent" }
                                    div { class: "relative",
                                        select {
                                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-600 appearance-none focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all cursor-pointer shadow-soft",
                                            value: if let Some(id) = verification_agent_id() { id.to_string() } else { "0".to_string() },
                                            onchange: move |e| {
                                                let id = e.value().parse::<i64>().unwrap_or(0);
                                                verification_agent_id.set(if id > 0 { Some(id) } else { None });
                                            },
                                            option { value: "0", "None (Auto-Grounding)" }
                                            if let Some(agent_list) = agents() {
                                                for agent in agent_list {
                                                    option { value: "{agent.id}", "{agent.name}" }
                                                }
                                            }
                                        }
                                        div { class: "absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400",
                                            MaterialIcon { name: "verified_user", size: 16 }
                                        }
                                    }
                                }
                                div { class: "space-y-2",
                                    label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Routing Priority" }
                                    input {
                                        r#type: "number",
                                        class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-600 focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all shadow-soft",
                                        value: "{routing_priority}",
                                        oninput: move |e| routing_priority.set(e.value().parse().unwrap_or(0)),
                                    }
                                }
                            }

                            // Display Controls (Evidence & Style)
                            div { class: "grid grid-cols-2 gap-6",
                                div { class: "flex items-center justify-between p-4 bg-slate-50 border border-slate-200 rounded-2xl shadow-soft",
                                    div { class: "flex items-center gap-4",
                                        div { class: "w-9 h-9 rounded-xl bg-orange-50 border border-orange-100 flex items-center justify-center text-orange-500",
                                            MaterialIcon { name: "visibility_off", size: 16, color: MaterialIconColor::Custom("inherit".into()) }
                                        }
                                        div {
                                            p { class: "text-xs font-black text-slate-900 tracking-tight", "Hide Evidence" }
                                            p { class: "text-[10px] text-slate-400 font-bold leading-tight", "Clean UI output." }
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: format!(
                                            "relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {}",
                                            if hide_evidence() { "bg-brand-blue" } else { "bg-slate-200" }
                                        ),
                                        onclick: move |_| hide_evidence.set(!hide_evidence()),
                                        span {
                                            class: format!(
                                                "pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {}",
                                                if hide_evidence() { "translate-x-4" } else { "translate-x-0" }
                                            )
                                        }
                                    }
                                }
                                div { class: "space-y-2",
                                    label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Report Style" }
                                    div { class: "relative",
                                        select {
                                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-600 appearance-none focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all cursor-pointer shadow-soft",
                                            value: "{report_style}",
                                            onchange: move |e| report_style.set(e.value()),
                                            option { value: "detailed", "Detailed" }
                                            option { value: "concise", "Concise" }
                                            option { value: "narrative", "Narrative" }
                                        }
                                        div { class: "absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400",
                                            MaterialIcon { name: "description", size: 16 }
                                        }
                                    }
                                }
                            }

                            // Bypass Synthesizer toggle
                            div { class: "flex items-center justify-between p-4 bg-slate-50 border border-slate-200 rounded-2xl shadow-soft",
                                div { class: "flex items-center gap-4",
                                    div { class: "w-9 h-9 rounded-xl bg-amber-50 border border-amber-100 flex items-center justify-center",
                                        MaterialIcon {
                                            name: "bolt",
                                            size: 16,
                                            color: MaterialIconColor::Custom("var(--color-amber-500)".into())
                                        }
                                    }
                                    div {
                                        p { class: "text-xs font-black text-slate-900 tracking-tight", "Bypass Mode" }
                                        p { class: "text-[10px] text-slate-400 font-bold leading-relaxed",
                                            "Return node output without synthesizer refinement."
                                        }
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: format!(
                                        "relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {}",
                                        if bypass_synthesizer() { "bg-brand-blue" } else { "bg-slate-200" }
                                    ),
                                    onclick: move |_| bypass_synthesizer.set(!bypass_synthesizer()),
                                    span {
                                        class: format!(
                                            "pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {}",
                                            if bypass_synthesizer() { "translate-x-4" } else { "translate-x-0" }
                                        )
                                    }
                                }
                            }
                        }
                    }

                    // ── AGENTS tab ─────────────────────────────────────
                    if active_tab() == ModalTab::Agents {
                        div { class: "animate-in fade-in duration-200",
                            if is_editing {
                                AgentAssignments {
                                    supervisor_id: editing_supervisor().unwrap().id,
                                    supervisor_agents: modal_supervisor_agents,
                                    agents,
                                }
                            } else {
                                div { class: "flex flex-col items-center justify-center py-16 px-6 bg-slate-50 border-2 border-dashed border-slate-200 rounded-[2rem] text-center",
                                    div { class: "w-14 h-14 bg-white rounded-2xl shadow-sm flex items-center justify-center mb-6 text-slate-300 border border-slate-100",
                                        MaterialIcon { name: "hub", size: 28 }
                                    }
                                    h4 { class: "text-sm font-black text-slate-900", "Deployment Required" }
                                    p { class: "text-[10px] text-slate-400 mt-2 font-bold uppercase tracking-widest max-w-[240px]",
                                        "Save node configuration to unlock fleet assignments."
                                    }
                                }
                            }
                        }
                    }

                    // ── STRATEGY tab ───────────────────────────────────
                    if active_tab() == ModalTab::Preamble {
                        div { class: "space-y-3 animate-in fade-in duration-200",
                            div { class: "flex items-center justify-between",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Strategy Preamble" }
                                span { class: "text-[9px] font-black text-slate-300 uppercase tracking-widest bg-slate-100 px-2 py-0.5 rounded",
                                    "{instructions_input().len()} characters"
                                }
                            }
                            textarea {
                                class: "w-full min-h-[280px] bg-slate-50 border border-slate-200 rounded-2xl px-5 py-4 text-sm font-medium leading-relaxed text-slate-600 focus:outline-none focus:ring-4 focus:ring-brand-blue/5 focus:border-brand-blue transition-all placeholder-slate-300 resize-none",
                                placeholder: "Define the core reasoning strategy and constraints for this supervisor...",
                                value: "{instructions_input}",
                                oninput: move |e| instructions_input.set(e.value()),
                                spellcheck: false,
                            }
                        }
                    }

                    // ── CODE tab ───────────────────────────────────────
                    if active_tab() == ModalTab::Code {
                        div { class: "space-y-3 animate-in fade-in duration-200",
                            div { class: "flex items-center justify-between",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Execution Logic" }
                                span { class: "px-2 py-0.5 bg-orange-500/10 text-orange-500 text-[9px] font-black rounded uppercase tracking-widest", "Rust Runtime" }
                            }
                            div { class: "bg-slate-900 rounded-2xl overflow-hidden border border-slate-800 shadow-xl relative",
                                textarea {
                                    class: "w-full min-h-[300px] p-6 bg-transparent text-slate-300 font-mono text-[11px] leading-relaxed outline-none resize-none custom-scrollbar",
                                    placeholder: "// Custom orchestration logic optional...",
                                    value: "{code_input}",
                                    oninput: move |e| code_input.set(e.value()),
                                    spellcheck: false,
                                }
                            }
                        }
                    }
                }

                // ── Footer ─────────────────────────────────────────────
                div { class: "flex items-center justify-between gap-3 px-10 py-6 border-t border-slate-100 bg-slate-50/50",
                    div { class: "flex items-center gap-1.5",
                        for dot_tab in [ModalTab::Basic, ModalTab::Agents, ModalTab::Preamble, ModalTab::Code] {
                            div {
                                class: format!(
                                    "rounded-full transition-all duration-500 {}",
                                    if active_tab() == dot_tab { "w-6 h-1.5 bg-brand-blue" }
                                    else { "w-1.5 h-1.5 bg-slate-200" }
                                )
                            }
                        }
                    }
                    div { class: "flex gap-4",
                        button {
                            class: "px-6 py-2.5 bg-white border border-slate-200 text-slate-500 rounded-xl transition-all text-xs font-bold uppercase tracking-widest hover:bg-slate-100",
                            onclick: move |_| show_modal.set(false),
                            "Cancel"
                        }
                        button {
                            class: "px-8 py-2.5 bg-brand-blue hover:bg-blue-500 text-white rounded-xl transition-all text-xs font-bold uppercase tracking-widest shadow-lg shadow-blue-100 active:scale-95 flex items-center gap-2",
                            onclick: move |_| handle_save.call(()),
                            MaterialIcon { name: "verified", size: 16 }
                            if is_editing { "Sync Node" } else { "Deploy Node" }
                        }
                    }
                }
            }
        }
    }
}
