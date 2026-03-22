use dioxus::prelude::*;
use dioxus::prelude::document::eval;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::Supervisor;

use crate::backend::{get_supervisor_agents, get_available_models};
use crate::state::use_global_state;
use crate::engine::utils::discovery::AgentInfo;
use super::agent_assignments::AgentAssignments;

// Re-use the same Prism highlight helper from the code editor
static PRISM_HIGHLIGHT_JS: &str = r#"
(function() { if (window.Prism) { window.Prism.highlightAll(); } })();
"#;

static MODAL_CODE_CSS: &str = r#"
.modal-prism-mirror,
.modal-prism-textarea {
    position: absolute;
    top: 0; left: 0;
    width: 100%; height: 100%;
    margin: 0;
    padding: 0.875rem 1rem;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.75rem;
    line-height: 1.65;
    tab-size: 4;
    white-space: pre;
    overflow: auto;
    box-sizing: border-box;
    border: none;
    border-radius: 0;
    letter-spacing: 0.01em;
}
.modal-prism-mirror {
    pointer-events: none;
    background: transparent !important;
    z-index: 1;
}
.modal-prism-textarea {
    background: transparent;
    color: transparent;
    caret-color: #e2e8f0;
    resize: none;
    z-index: 2;
    outline: none;
    -webkit-text-fill-color: transparent;
}
.modal-code-body {
    position: relative;
    min-height: 220px;
    overflow: auto;
}
"#;

#[derive(Clone, PartialEq)]
enum ModalTab { Basic, Preamble, Code, Agents }

#[component]
pub fn SupervisorModal(
    mut show_modal: Signal<bool>,
    editing_supervisor: Signal<Option<Supervisor>>,
    // ── Basic fields ──────────────────────────────────────────
    mut name_input: Signal<String>,
    mut description_input: Signal<String>,
    mut domain_input: Signal<String>,
    // ── Extended fields ───────────────────────────────────────
    mut instructions_input: Signal<String>,
    mut model_input: Signal<String>,
    mut code_input: Signal<String>,
    mut bypass_synthesizer: Signal<bool>,
    // ── Available models (passed from workshop) ───────────────
    available_models: Signal<Vec<String>>,
    // ── Global tools (passed from workshop) ──────────────────
    agents: Resource<Vec<AgentInfo>>,
    // ── Save handler ─────────────────────────────────────────
    handle_save: EventHandler<()>,
) -> Element {
    let state = use_global_state();
    let mut active_tab = use_signal(|| ModalTab::Basic);
    let is_editing = editing_supervisor().is_some();

    // ── Local resource for agents assigned to THIS supervisor ────────────────
    let mut modal_supervisor_agents = use_resource(move || {
        let state = state;
        let s = editing_supervisor();
        async move {
            if let Some(supervisor) = s {
                let db = state.read().db.clone();
                get_supervisor_agents(&db, supervisor.id).await.unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    // Re-highlight whenever code changes or tab switches
    use_effect(move || {
        let _ = code_input();
        let _ = active_tab();
        eval(PRISM_HIGHLIGHT_JS);
    });

    rsx! {
        // CDN assets
        script { src: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/prism.min.js" }
        script { src: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-rust.min.js" }
        link {
            rel: "stylesheet",
            href: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css"
        }
        style { {MODAL_CODE_CSS} }

        // ── Backdrop ───────────────────────────────────────────────────────
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-[#0F172A]/50 backdrop-blur-sm animate-in fade-in duration-200",
            // Click-outside to close
            onclick: move |_| show_modal.set(false),

            // ── Dialog ────────────────────────────────────────────────────
            div {
                class: "w-full max-w-xl bg-white border border-[#E2E8F0] rounded-2xl shadow-2xl flex flex-col animate-in zoom-in-95 duration-200 overflow-hidden",
                // Stop backdrop click propagating through the card
                onclick: move |e| e.stop_propagation(),

                // ── Header ─────────────────────────────────────────────
                div { class: "flex items-center justify-between px-6 pt-6 pb-4",
                    div {
                        div { class: "flex items-center gap-3",
                            div { class: "w-9 h-9 rounded-xl bg-blue-50 border border-blue-100 flex items-center justify-center",
                                MaterialIcon {
                                    name: "account_tree",
                                    size: 18,
                                    color: MaterialIconColor::Custom("#3b82f6".into())
                                }
                            }
                            div {
                                h2 { class: "text-base font-bold text-[#0F172A]",
                                    if is_editing { "Edit Supervisor" } else { "New Supervisor" }
                                }
                                p { class: "text-slate-400 text-xs mt-0.5",
                                    if is_editing {
                                        "Update identity, orchestration agents, and logic."
                                    } else {
                                        "Configure a new orchestrator for your agent hierarchy."
                                    }
                                }
                            }
                        }
                    }
                    button {
                        class: "p-1.5 rounded-lg hover:bg-slate-100 text-slate-400 hover:text-slate-600 transition-colors",
                        onclick: move |_| show_modal.set(false),
                        MaterialIcon { name: "close", size: 18 }
                    }
                }

                // ── Tab bar ────────────────────────────────────────────
                div { class: "flex gap-1 mx-6 mb-1 bg-slate-50 border border-[#E2E8F0] rounded-xl p-1",
                    for (tab, label, icon) in [
                        (ModalTab::Basic,    "Identity",  "badge"),
                        (ModalTab::Agents,   "Agents",    "hub"),
                        (ModalTab::Preamble, "Preamble",  "description"),
                        (ModalTab::Code,     "Logic",     "code"),
                    ] {
                        {
                            let is_active = active_tab() == tab;
                            let tab_c = tab.clone();
                            rsx! {
                                button {
                                    key: "{label}",
                                    class: format!(
                                        "flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-[10px] font-bold transition-all {}",
                                        if is_active { "bg-white text-[#0F172A] shadow-soft border border-[#E2E8F0]" }
                                        else { "text-slate-400 hover:text-slate-600" }
                                    ),
                                    onclick: move |_| active_tab.set(tab_c.clone()),
                                    MaterialIcon {
                                        name: icon,
                                        size: 14,
                                        color: if is_active {
                                            MaterialIconColor::Custom("#3b82f6".into())
                                        } else {
                                            MaterialIconColor::Custom("#94a3b8".into())
                                        }
                                    }
                                    span { class: "uppercase tracking-wider", "{label}" }
                                }
                            }
                        }
                    }
                }

                // ── Tab content ────────────────────────────────────────
                div { class: "flex-1 px-6 py-4 space-y-4 overflow-y-auto max-h-[420px] custom-scrollbar",

                    // ── IDENTITY tab ───────────────────────────────────
                    if active_tab() == ModalTab::Basic {
                        div { class: "space-y-4 animate-in fade-in duration-150",

                            // Name
                            div { class: "space-y-1.5",
                                label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Display Name" }
                                input {
                                    class: "w-full bg-[#F8FAFC] border border-[#E2E8F0] rounded-xl px-4 py-3 text-sm text-[#0F172A] focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6]/50 transition-all placeholder-slate-300",
                                    placeholder: "e.g. Research Supervisor",
                                    value: "{name_input}",
                                    oninput: move |e| name_input.set(e.value()),
                                }
                            }

                            // Description
                            div { class: "space-y-1.5",
                                label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Description" }
                                input {
                                    class: "w-full bg-[#F8FAFC] border border-[#E2E8F0] rounded-xl px-4 py-3 text-sm text-[#0F172A] focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6]/50 transition-all placeholder-slate-300",
                                    placeholder: "e.g. Handles web research tasks",
                                    value: "{description_input}",
                                    oninput: move |e| description_input.set(e.value()),
                                }
                            }

                            // Domain + Model side by side
                            div { class: "grid grid-cols-2 gap-4",
                                div { class: "space-y-1.5",
                                    label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Execution Domain" }
                                    input {
                                        class: "w-full bg-[#F8FAFC] border border-[#E2E8F0] rounded-xl px-4 py-3 text-sm text-[#0F172A] focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6]/50 transition-all placeholder-slate-300",
                                        placeholder: "e.g. web, office",
                                        value: "{domain_input}",
                                        oninput: move |e| domain_input.set(e.value()),
                                    }
                                }
                                div { class: "space-y-1.5",
                                    label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Model" }
                                    div { class: "relative",
                                        select {
                                            class: "w-full bg-[#F8FAFC] border border-[#E2E8F0] rounded-xl px-4 py-3 text-sm text-[#0F172A] appearance-none focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6]/50 transition-all cursor-pointer",
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

                            // Bypass Synthesizer toggle
                            div { class: "flex items-center justify-between p-3 bg-[#F8FAFC] border border-[#E2E8F0] rounded-xl",
                                div { class: "flex items-center gap-3",
                                    div { class: "w-7 h-7 rounded-lg bg-amber-50 border border-amber-100 flex items-center justify-center",
                                        MaterialIcon {
                                            name: "flash_on",
                                            size: 14,
                                            color: MaterialIconColor::Custom("#f59e0b".into())
                                        }
                                    }
                                    div {
                                        p { class: "text-xs font-semibold text-[#0F172A]", "Bypass Synthesizer" }
                                        p { class: "text-[10px] text-slate-400 leading-relaxed mt-0.5",
                                            "Return this supervisor's output directly, skipping the Synthesizer LLM."
                                        }
                                    }
                                }
                                // Toggle switch
                                button {
                                    r#type: "button",
                                    class: format!(
                                        "relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {}",
                                        if bypass_synthesizer() { "bg-blue-500" } else { "bg-slate-200" }
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
                        div { class: "space-y-6 animate-in fade-in duration-150 py-2",
                            if let Some(sv) = editing_supervisor() {
                                AgentAssignments {
                                    supervisor_id: sv.id,
                                    supervisor_agents: modal_supervisor_agents,
                                    agents,
                                }
                            } else {
                                div { class: "flex flex-col items-center justify-center py-12 px-6 bg-slate-50 border border-dashed border-slate-200 rounded-2xl text-center",
                                    div { class: "w-12 h-12 bg-white rounded-full shadow-sm flex items-center justify-center mb-4 text-slate-300",
                                        MaterialIcon { name: "hub", size: 24 }
                                    }
                                    h4 { class: "text-sm font-bold text-slate-900", "Initialize Supervisor First" }
                                    p { class: "text-xs text-slate-400 mt-2 max-w-[240px] leading-relaxed",
                                        "To assign agents, you must first create this supervisor. Save the basic configuration to continue."
                                    }
                                }
                            }
                        }
                    }

                    // ── PREAMBLE tab ───────────────────────────────────
                    if active_tab() == ModalTab::Preamble {
                        div { class: "space-y-2 animate-in fade-in duration-150",
                            div { class: "flex items-center justify-between",
                                label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Base Instructions" }
                                span { class: "text-[10px] text-slate-300 font-mono",
                                    "{instructions_input().len()} chars"
                                }
                            }
                            textarea {
                                class: "w-full min-h-[300px] bg-[#F8FAFC] border border-[#E2E8F0] rounded-xl px-4 py-3 text-sm font-mono text-[#0F172A] focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6]/50 transition-all placeholder-slate-300 resize-y leading-relaxed",
                                placeholder: "Define the core identity, reasoning loop, and behavioural rules for this supervisor...",
                                value: "{instructions_input}",
                                oninput: move |e| instructions_input.set(e.value()),
                                spellcheck: false,
                            }
                            p { class: "text-[10px] text-slate-300 leading-relaxed",
                                "This preamble is injected at the start of every prompt sent to this supervisor."
                            }
                        }
                    }

                    // ── CODE tab ───────────────────────────────────────
                    if active_tab() == ModalTab::Code {
                        div { class: "space-y-2 animate-in fade-in duration-150",
                            div { class: "flex items-center justify-between",
                                label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Code Logic" }
                                div { class: "flex items-center gap-2",
                                    span { class: "px-2 py-0.5 bg-orange-500/10 text-orange-400 text-[9px] font-bold rounded uppercase tracking-wider", "Rust" }
                                    span { class: "text-[10px] text-slate-300 font-mono",
                                        "{code_input().lines().count()} lines"
                                    }
                                }
                            }
                            // Prism dual-layer editor
                            div { class: "bg-[#1e1e1e] rounded-xl overflow-hidden border border-white/5",
                                // Mini VS-Code titlebar
                                div { class: "flex items-center gap-2 px-4 py-2 bg-[#252526] border-b border-white/5",
                                    div { class: "flex gap-1",
                                        div { class: "w-2.5 h-2.5 rounded-full bg-[#ff5f57]" }
                                        div { class: "w-2.5 h-2.5 rounded-full bg-[#febc2e]" }
                                        div { class: "w-2.5 h-2.5 rounded-full bg-[#28c840]" }
                                    }
                                    span { class: "text-white/30 text-[10px] font-mono ml-1", "supervisor.rs" }
                                }
                                div { class: "modal-code-body",
                                    pre { class: "modal-prism-mirror language-rust",
                                        code { class: "language-rust", "{code_input}\n" }
                                    }
                                    textarea {
                                        class: "modal-prism-textarea",
                                        placeholder: "// Write supervisor logic here...",
                                        value: "{code_input}",
                                        oninput: move |e| {
                                            code_input.set(e.value());
                                            eval(PRISM_HIGHLIGHT_JS);
                                        },
                                        spellcheck: false,
                                        autocomplete: "off",
                                        autocorrect: "off",
                                    }
                                }
                            }
                            p { class: "text-[10px] text-slate-300 leading-relaxed",
                                "Optional Rust scripting logic executed by the supervisor runtime."
                            }
                        }
                    }
                }

                // ── Footer ─────────────────────────────────────────────
                div { class: "flex items-center justify-between gap-3 px-6 py-4 border-t border-[#E2E8F0] bg-slate-50/60",
                    // Tab progress dots
                    div { class: "flex items-center gap-1.5",
                        for (dot_tab, _) in [
                            (ModalTab::Basic, ""),
                            (ModalTab::Agents, ""),
                            (ModalTab::Preamble, ""),
                            (ModalTab::Code, ""),
                        ] {
                            div {
                                class: format!(
                                    "rounded-full transition-all duration-200 {}",
                                    if active_tab() == dot_tab {
                                        "w-4 h-1.5 bg-[#3B82F6]"
                                    } else {
                                        "w-1.5 h-1.5 bg-slate-200"
                                    }
                                )
                            }
                        }
                    }
                    // Buttons
                    div { class: "flex gap-2",
                        button {
                            class: "px-5 py-2.5 bg-slate-100 hover:bg-slate-200 text-slate-600 rounded-xl transition-colors text-sm font-semibold",
                            onclick: move |_| show_modal.set(false),
                            "Cancel"
                        }
                        button {
                            class: "px-5 py-2.5 bg-[#3B82F6] hover:bg-blue-500 text-white rounded-xl transition-colors text-sm font-semibold shadow-sm flex items-center gap-2",
                            onclick: move |_| handle_save.call(()),
                            MaterialIcon { name: "save", size: 16, color: MaterialIconColor::Light }
                            if is_editing { "Save Changes" } else { "Create Supervisor" }
                        }
                    }
                }
            }
        }
    }
}