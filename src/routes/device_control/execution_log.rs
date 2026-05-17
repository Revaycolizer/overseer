use dioxus::prelude::*;

#[component]
pub fn ExecutionLog() -> Element {
    rsx! {
        aside { class: "w-80 bg-surface-container border-r border-outline-variant/10 flex flex-col min-h-0 shadow-inner",
            // Header
            div { class: "p-4 flex items-center justify-between bg-surface-container-low border-b border-outline-variant/10",
                h2 { class: "text-xs font-bold uppercase tracking-widest text-on-surface-variant flex items-center gap-2",
                    span { class: "material-symbols-outlined text-sm text-secondary", "terminal" }
                    "EXECUTION LOG"
                }
                span { class: "text-[10px] font-mono text-outline", "v2.4.0" }
            }
            // Log entries
            div { class: "flex-1 overflow-y-auto p-4 space-y-4 font-mono text-[11px] leading-relaxed",
                // Session Start
                div { class: "space-y-1",
                    div { class: "flex gap-2 text-outline",
                        span { "[14:22:01]" }
                        span { class: "text-secondary-container", "INITIATE" }
                    }
                    p { class: "text-on-surface-variant ml-4", "Session: \"Market Analysis Automation\"" }
                }
                // Browser Event
                div { class: "space-y-1",
                    div { class: "flex gap-2 text-outline",
                        span { "[14:22:05]" }
                        span { class: "text-on-tertiary-container", "ACTIVE" }
                    }
                    p { class: "text-on-surface font-bold ml-4", "AGENT: Opening Google Chrome..." }
                }
                // Search overlay
                div { class: "p-3 bg-surface-container-lowest rounded border border-outline-variant/20 shadow-sm ml-2",
                    p { class: "text-on-surface-variant mb-2 flex items-center gap-2 italic",
                        span { class: "material-symbols-outlined text-[12px]", "search" }
                        "Searching..."
                    }
                    div { class: "h-1 w-full bg-surface-container-high rounded-full overflow-hidden",
                        div { class: "h-full bg-secondary w-full" }
                    }
                }
                // Data Step
                div { class: "space-y-1",
                    div { class: "flex gap-2 text-outline",
                        span { "[14:23:12]" }
                        span { class: "text-secondary-container", "PROCESS" }
                    }
                    p { class: "text-on-surface font-bold ml-4", "AGENT: Extracting tabular data" }
                    div { class: "ml-8 text-outline space-y-0.5",
                        p { "> mapping: table_data_01" }
                        p { "> mapping: column_header_id" }
                    }
                }
                // Logic Repair
                div { class: "p-3 bg-primary-container/10 border border-primary-container/20 rounded-lg ml-2",
                    div { class: "flex items-start gap-2 mb-2",
                        span { class: "material-symbols-outlined text-blue-500 text-sm", "psychology" }
                        p { class: "text-[10px] text-on-surface-variant leading-tight", "Mismatch detected. Auto-patching with script v2.1..." }
                    }
                    div { class: "flex gap-1",
                        span { class: "px-1.5 py-0.5 bg-blue-100 text-[9px] font-bold text-blue-700 rounded border border-blue-200", "JS_INJECT" }
                        span { class: "px-1.5 py-0.5 bg-green-100 text-[9px] font-bold text-green-700 rounded border border-green-200", "REPAIRED" }
                    }
                }
                // Current State
                div { class: "space-y-1",
                    div { class: "flex gap-2 text-secondary font-bold",
                        span { "[14:24:00]" }
                        span { class: "animate-pulse", "AWAIT_INPUT" }
                    }
                    p { class: "text-on-surface font-bold ml-4", "AGENT: Requesting manual review of Cell B42..." }
                }
            }
            // Footer telemetry
            div { class: "p-4 bg-surface-container-high/50 border-t border-outline-variant/10",
                div { class: "flex justify-between items-center text-[10px] text-outline font-bold",
                    span { "CPU: 12%" }
                    span { "RAM: 4.2GB" }
                    span { "NET: 42ms" }
                }
            }
        }
    }
}
