use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::Supervisor;

#[component]
pub fn PreambleEditor(
    mut selected_supervisor: Signal<Option<Supervisor>>,
    mut local_instructions: Signal<String>,
) -> Element {
    let mut fullscreen = use_signal(|| false);

    let Some(supervisor) = selected_supervisor() else {
        return rsx! {};
    };

    rsx! {
        // ── Fullscreen overlay ─────────────────────────────────────────────
        if fullscreen() {
            div { class: "fixed inset-0 z-50 bg-[#0F172A] flex flex-col animate-in fade-in duration-150",
                // Toolbar
                div { class: "flex items-center justify-between px-6 py-4 border-b border-white/10 bg-[#0F172A]",
                    div { class: "flex items-center gap-3",
                        MaterialIcon {
                            name: "description",
                            size: 20,
                            color: MaterialIconColor::Custom("#94a3b8".into())
                        }
                        span {
                            class: "text-white/70 text-sm font-bold uppercase tracking-widest",
                            "Base Instructions — Preamble"
                        }
                        span {
                            class: "px-2 py-0.5 bg-white/5 text-white/30 rounded text-[10px] font-mono",
                            "{local_instructions().len()} chars · {local_instructions().lines().count()} lines"
                        }
                    }
                    button {
                        class: "flex items-center gap-2 px-4 py-2 bg-white/10 hover:bg-white/15 text-white rounded-lg text-xs font-bold transition-colors",
                        onclick: move |_| fullscreen.set(false),
                        MaterialIcon { name: "fullscreen_exit", size: 18, color: MaterialIconColor::Light }
                        "Exit Fullscreen"
                    }
                }
                // Editor
                textarea {
                    class: "flex-1 bg-transparent text-slate-200 font-mono text-sm p-8 resize-none focus:outline-none placeholder-slate-600 leading-relaxed",
                    placeholder: "Define the core identity, reasoning loop, and behavioural rules for this supervisor...",
                    value: "{local_instructions}",
                    oninput: {
                        let mut sv = supervisor.clone();
                        move |e| {
                            local_instructions.set(e.value());
                            sv.instructions = Some(e.value());
                            selected_supervisor.set(Some(sv.clone()));
                        }
                    },
                    spellcheck: false,
                }
            }
        }

        // ── Normal card ────────────────────────────────────────────────────
        div { class: "bg-white border border-[#E2E8F0] rounded-2xl shadow-soft overflow-hidden",

            // Toolbar
            div { class: "flex items-center justify-between px-5 py-3 bg-slate-50 border-b border-[#E2E8F0]",
                div { class: "flex items-center gap-2",
                    MaterialIcon {
                        name: "description",
                        size: 16,
                        color: MaterialIconColor::Custom("#94a3b8".into())
                    }
                    span {
                        class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                        "Base Instructions (Preamble)"
                    }
                }
                div { class: "flex items-center gap-3",
                    span { class: "text-[10px] text-slate-300 font-mono",
                        "{local_instructions().len()} chars · {local_instructions().lines().count()} lines"
                    }
                    button {
                        class: "p-1.5 rounded-md hover:bg-slate-100 text-slate-400 hover:text-slate-700 transition-colors",
                        title: "Fullscreen",
                        onclick: move |_| fullscreen.set(true),
                        MaterialIcon { name: "fullscreen", size: 18 }
                    }
                }
            }

            // Textarea
            textarea {
                class: "w-full min-h-[320px] bg-white text-[#0F172A] font-mono text-sm p-6 resize-y focus:outline-none placeholder-slate-300 leading-relaxed border-none",
                placeholder: "Define the core identity, reasoning loop, and behavioural rules for this supervisor...",
                value: "{local_instructions}",
                oninput: {
                    let mut sv = supervisor.clone();
                    move |e| {
                        local_instructions.set(e.value());
                        sv.instructions = Some(e.value());
                        selected_supervisor.set(Some(sv.clone()));
                    }
                },
                spellcheck: false,
            }
        }
    }
}
