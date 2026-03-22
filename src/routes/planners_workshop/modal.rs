use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::Planner;

#[derive(Clone, PartialEq)]
enum ModalTab { Basic, Preamble, Code }

#[component]
pub fn PlannerModal(
    mut show_modal: Signal<bool>,
    editing_planner: Signal<Option<Planner>>,
    // ── Basic fields ──────────────────────────────────────────
    mut name_input: Signal<String>,
    mut description_input: Signal<String>,
    mut domain_input: Signal<String>,
    // ── Extended fields ───────────────────────────────────────
    mut instructions_input: Signal<String>,
    mut code_input: Signal<String>,
    // ── Save handler ─────────────────────────────────────────
    handle_save: EventHandler<()>,
) -> Element {
    let mut active_tab = use_signal(|| ModalTab::Basic);
    let is_editing = editing_planner().is_some();

    rsx! {
        // ── Backdrop ───────────────────────────────────────────────────────
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-sm animate-in fade-in duration-200",
            onclick: move |_| show_modal.set(false),

            // ── Dialog ────────────────────────────────────────────────────
            div {
                class: "w-full max-w-xl bg-white border border-slate-200 rounded-2xl shadow-2xl flex flex-col animate-in zoom-in-95 duration-200 overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // ── Header ─────────────────────────────────────────────
                div { class: "flex items-center justify-between px-6 pt-6 pb-4",
                    div {
                        div { class: "flex items-center gap-3",
                            div { class: "w-9 h-9 rounded-xl bg-blue-50 border border-blue-100 flex items-center justify-center",
                                MaterialIcon {
                                    name: "architecture",
                                    size: 18,
                                    color: MaterialIconColor::Custom("var(--color-brand-blue)".into())
                                }
                            }
                            div {
                                h2 { class: "text-base font-bold text-slate-900",
                                    if is_editing { "Modify Planner Strategy" } else { "Initialize Planner Node" }
                                }
                                p { class: "text-slate-400 text-xs mt-0.5",
                                    if is_editing {
                                        "Refine the strategic objectives and logic for this domain."
                                    } else {
                                        "Configure a new hierarchical planner for domain-specific orchestration."
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
                div { class: "flex gap-1 mx-6 mb-1 bg-slate-50 border border-slate-200 rounded-xl p-1",
                    for (tab, label, icon) in [
                        (ModalTab::Basic,    "Identity",  "badge"),
                        (ModalTab::Preamble, "Strategy",  "description"),
                        (ModalTab::Code,     "Workflow",  "code"),
                    ] {
                        {
                            let is_active = active_tab() == tab;
                            let tab_c = tab.clone();
                            rsx! {
                                button {
                                    key: "{label}",
                                    class: format!(
                                        "flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-[10px] font-bold transition-all {}",
                                        if is_active { "bg-brand-blue text-white shadow-md shadow-blue-100" }
                                        else { "text-slate-400 hover:text-slate-600" }
                                    ),
                                    onclick: move |_| active_tab.set(tab_c.clone()),
                                    MaterialIcon {
                                        name: icon,
                                        size: 14,
                                        color: if is_active {
                                            MaterialIconColor::Custom("var(--color-brand-blue)".into())
                                        } else {
                                            MaterialIconColor::Custom("var(--color-slate-400)".into())
                                        }
                                    }
                                    span { class: "uppercase tracking-wider", "{label}" }
                                }
                            }
                        }
                    }
                }

                // ── Tab content ────────────────────────────────────────
                div { class: "flex-1 px-6 py-6 space-y-4 overflow-y-auto max-h-[420px] custom-scrollbar",

                    if active_tab() == ModalTab::Basic {
                        div { class: "space-y-4 animate-in fade-in duration-150",
                            div { class: "space-y-1.5",
                                label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Planner Designation" }
                                input {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-brand-blue/20 focus:border-brand-blue/50 transition-all placeholder-slate-300",
                                    placeholder: "e.g. Research Orchestrator",
                                    value: "{name_input}",
                                    oninput: move |e| name_input.set(e.value()),
                                }
                            }

                            div { class: "space-y-1.5",
                                label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Strategy Description" }
                                input {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-brand-blue/20 focus:border-brand-blue/50 transition-all placeholder-slate-300",
                                    placeholder: "Describe the high-level goal of this planner...",
                                    value: "{description_input}",
                                    oninput: move |e| description_input.set(e.value()),
                                }
                            }

                            div { class: "space-y-1.5",
                                label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Domain Scope" }
                                input {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-mono text-slate-900 focus:outline-none focus:ring-2 focus:ring-brand-blue/20 focus:border-brand-blue/50 transition-all placeholder-slate-300",
                                    placeholder: "e.g. general_research",
                                    value: "{domain_input}",
                                    oninput: move |e| domain_input.set(e.value()),
                                }
                            }
                        }
                    }

                    if active_tab() == ModalTab::Preamble {
                        div { class: "space-y-2 animate-in fade-in duration-150",
                            textarea {
                                class: "w-full min-h-[300px] bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-xs leading-relaxed text-slate-900 focus:outline-none focus:ring-2 focus:ring-brand-blue/20 focus:border-brand-blue/50 transition-all placeholder-slate-300 resize-y",
                                placeholder: "Define the core planning logic and strategy guidelines...",
                                value: "{instructions_input}",
                                oninput: move |e| instructions_input.set(e.value()),
                                spellcheck: false,
                            }
                        }
                    }

                    if active_tab() == ModalTab::Code {
                        div { class: "animate-in fade-in duration-150",
                            div { class: "bg-slate-900 rounded-xl overflow-hidden border border-slate-800 shadow-xl",
                                textarea {
                                    class: "w-full min-h-[300px] p-6 bg-transparent text-slate-300 font-mono text-[10px] leading-relaxed outline-none resize-none",
                                    value: "{code_input}",
                                    oninput: move |e| code_input.set(e.value()),
                                    spellcheck: false,
                                    placeholder: "// Custom workflow orchestration logic...",
                                }
                            }
                        }
                    }
                }

                // ── Footer ─────────────────────────────────────────────
                div { class: "flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-200 bg-slate-50/60",
                    button {
                        class: "px-5 py-2.5 bg-slate-100 hover:bg-slate-200 text-slate-600 rounded-xl transition-colors text-xs font-bold uppercase tracking-widest",
                        onclick: move |_| show_modal.set(false),
                        "Cancel"
                    }
                    button {
                        class: "px-5 py-2.5 bg-brand-blue hover:bg-blue-500 text-white rounded-xl transition-colors text-xs font-bold uppercase tracking-widest shadow-md shadow-blue-100 flex items-center gap-2",
                        onclick: move |_| handle_save.call(()),
                        MaterialIcon { name: "save", size: 14, color: MaterialIconColor::Light }
                        if is_editing { "Update Strategy" } else { "Initialize Planner" }
                    }
                }
            }
        }
    }
}
