use dioxus::prelude::*;
use dioxus::prelude::document::eval;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::Supervisor;

use super::test_panel::TestPanel;

pub static PRISM_HIGHLIGHT_JS: &str = r#"
(function() { if (window.Prism) { window.Prism.highlightAll(); } })();
"#;

pub static CODE_EDITOR_CSS: &str = r#"
/* ── Prism dual-layer editor ────────────────────────── */
.prism-mirror,
.prism-textarea {
    position: absolute;
    top: 0; left: 0;
    width: 100%;
    height: 100%;
    min-height: 400px;
    margin: 0;
    padding: 1.25rem 1.5rem;
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 0.8125rem;
    line-height: 1.7;
    tab-size: 4;
    white-space: pre;
    overflow: auto;
    box-sizing: border-box;
    border: none;
    border-radius: 0;
    letter-spacing: 0.01em;
}
.prism-mirror {
    pointer-events: none;
    background: transparent !important;
    z-index: 1;
}
.prism-textarea {
    background: transparent;
    color: transparent;
    caret-color: #e2e8f0;
    resize: none;
    z-index: 2;
    outline: none;
    -webkit-text-fill-color: transparent;
}
.code-editor-body {
    position: relative;
    min-height: 400px;
    overflow: auto;
}
.token.comment { color: #6a737d !important; }
"#;

// ─── Sub-component: the raw editor layers (shared between normal & fullscreen)
#[component]
fn EditorLayers(
    mut local_code: Signal<String>,
    mut selected_supervisor: Signal<Option<crate::db::Supervisor>>,
) -> Element {
    rsx! {
        div { class: "code-editor-body",
            // Syntax-highlighted mirror (Prism)
            pre { class: "prism-mirror language-rust",
                code { class: "language-rust",
                    "{local_code}\n"
                }
            }
            // Transparent editable textarea on top
            textarea {
                class: "prism-textarea",
                placeholder: "// Write supervisor logic here...",
                value: "{local_code}",
                oninput: {
                    move |e| {
                        local_code.set(e.value());
                        if let Some(mut sv) = selected_supervisor() {
                            sv.code = Some(e.value());
                            selected_supervisor.set(Some(sv));
                        }
                        eval(PRISM_HIGHLIGHT_JS);
                    }
                },
                spellcheck: false,
                autocomplete: "off",
                autocorrect: "off",
            }
        }
    }
}

// ─── Main code editor component ─────────────────────────────────────────────
#[component]
pub fn CodeEditor(
    mut selected_supervisor: Signal<Option<Supervisor>>,
    mut local_code: Signal<String>,
) -> Element {
    let mut fullscreen    = use_signal(|| false);
    let mut show_test     = use_signal(|| false);

    // Re-highlight on every code change
    use_effect(move || {
        let _ = local_code();
        eval(PRISM_HIGHLIGHT_JS);
    });

    let Some(supervisor) = selected_supervisor() else {
        return rsx! {};
    };

    let filename = format!("supervisor_{}.rs", supervisor.id);
    let line_count = local_code().lines().count();

    rsx! {
        // ── CDN assets (idempotent — browser deduplicates) ──────────────
        script { src: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/prism.min.js" }
        script { src: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-rust.min.js" }
        link {
            rel: "stylesheet",
            href: "https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css"
        }
        style { {CODE_EDITOR_CSS} }

        // ── Fullscreen overlay ─────────────────────────────────────────
        if fullscreen() {
            div { class: "fixed inset-0 z-50 bg-[#1e1e1e] flex flex-col animate-in fade-in duration-150",

                // Fullscreen toolbar
                div { class: "flex items-center justify-between px-6 py-3 bg-[#252526] border-b border-white/10 shrink-0",
                    // Left: traffic lights + filename
                    div { class: "flex items-center gap-3",
                        div { class: "flex gap-1.5",
                            div { class: "w-3 h-3 rounded-full bg-[#ff5f57]" }
                            div { class: "w-3 h-3 rounded-full bg-[#febc2e]" }
                            div { class: "w-3 h-3 rounded-full bg-[#28c840]" }
                        }
                        span { class: "text-white/40 text-xs font-mono ml-2", "{filename}" }
                        span {
                            class: "px-2 py-0.5 bg-orange-500/20 text-orange-300 text-[9px] font-bold rounded uppercase tracking-wider",
                            "Rust"
                        }
                    }
                    // Right: test toggle + exit
                    div { class: "flex items-center gap-2",
                        button {
                            class: format!(
                                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold transition-colors {}",
                                if show_test() { "bg-[#3B82F6] text-white" }
                                else { "bg-white/10 hover:bg-white/15 text-white/70" }
                            ),
                            onclick: move |_| show_test.set(!show_test()),
                            MaterialIcon { name: "science", size: 15, color: MaterialIconColor::Light }
                            "Test"
                        }
                        button {
                            class: "flex items-center gap-2 px-3 py-1.5 bg-white/10 hover:bg-white/20 text-white rounded-lg text-xs font-bold transition-colors",
                            onclick: move |_| fullscreen.set(false),
                            MaterialIcon { name: "fullscreen_exit", size: 16, color: MaterialIconColor::Light }
                            "Exit"
                        }
                    }
                }

                // Fullscreen body: editor + optional test side-drawer
                div { class: "flex-1 flex overflow-hidden",

                    // Editor
                    div { class: "flex-1 overflow-auto bg-[#1e1e1e]",
                        EditorLayers { local_code, selected_supervisor }
                    }

                    // Test side-drawer
                    if show_test() {
                        div { class: "w-96 bg-[#0f172a] border-l border-white/10 flex flex-col shrink-0",
                            TestPanel {
                                supervisor_id: supervisor.id,
                                local_code,
                                on_close: None,
                            }
                        }
                    }
                }
            }
        }

        // ── Normal card ────────────────────────────────────────────────
        div { class: "space-y-4",

            // Editor card
            div { class: "bg-[#1e1e1e] rounded-2xl overflow-hidden border border-white/5 shadow-xl",

                // Toolbar
                div { class: "flex items-center justify-between px-5 py-3 bg-[#252526] border-b border-white/5 shrink-0",
                    // Left: traffic lights + filename + language badge
                    div { class: "flex items-center gap-3",
                        div { class: "flex gap-1.5",
                            div { class: "w-3 h-3 rounded-full bg-[#ff5f57]" }
                            div { class: "w-3 h-3 rounded-full bg-[#febc2e]" }
                            div { class: "w-3 h-3 rounded-full bg-[#28c840]" }
                        }
                        span { class: "text-white/40 text-xs font-mono", "{filename}" }
                        span {
                            class: "px-2 py-0.5 bg-orange-500/20 text-orange-300 text-[9px] font-bold rounded uppercase tracking-wider",
                            "Rust"
                        }
                    }
                    // Right: line count + test toggle + fullscreen
                    div { class: "flex items-center gap-2",
                        span { class: "text-white/20 text-[10px] font-mono", "{line_count} lines" }
                        button {
                            class: format!(
                                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold transition-colors {}",
                                if show_test() { "bg-[#3B82F6] text-white" }
                                else { "bg-white/10 hover:bg-white/15 text-white/70" }
                            ),
                            onclick: move |_| show_test.set(!show_test()),
                            MaterialIcon { name: "science", size: 15, color: MaterialIconColor::Light }
                            "Test"
                        }
                        button {
                            class: "p-1.5 rounded-md bg-white/5 hover:bg-white/15 text-white/50 hover:text-white transition-colors",
                            title: "Fullscreen editor",
                            onclick: move |_| fullscreen.set(true),
                            MaterialIcon { name: "fullscreen", size: 18, color: MaterialIconColor::Light }
                        }
                    }
                }

                // Editor body
                EditorLayers { local_code, selected_supervisor }
            }

            // Inline test panel (below editor)
            if show_test() {
                div { class: "bg-white border border-[#E2E8F0] rounded-2xl shadow-soft overflow-hidden animate-in slide-in-from-top-2 duration-200",
                    TestPanel {
                        supervisor_id: supervisor.id,
                        local_code,
                        on_close: EventHandler::new(move |_| show_test.set(false)),
                    }
                }
            }
        }
    }
}
