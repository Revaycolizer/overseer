use dioxus::prelude::*;

#[component]
pub fn LogList() -> Element {
    rsx! {
        section { class: "flex-1 overflow-hidden flex flex-col p-6",
            div { class: "flex items-center justify-between mb-4 shrink-0",
                div { class: "flex items-center gap-2",
                    h2 { class: "text-sm font-bold uppercase tracking-widest text-on-surface-variant", "Interaction Log" }
                    span { class: "bg-surface-container-highest text-on-surface-variant text-[10px] font-bold px-1.5 py-0.5 rounded", "v2.4.0" }
                }
                div { class: "flex items-center gap-2",
                    button { class: "text-xs font-bold text-secondary hover:underline flex items-center gap-1",
                        "Filter"
                        span { class: "material-symbols-outlined text-sm", "filter_list" }
                    }
                    span { class: "text-outline-variant", "|" }
                    button { class: "text-xs font-bold text-secondary hover:underline flex items-center gap-1",
                        "Export"
                        span { class: "material-symbols-outlined text-sm", "download" }
                    }
                }
            }
            div { class: "flex-1 overflow-y-auto custom-scrollbar pr-2 space-y-4",
                LogItemBash {}
                LogItemDocument {}
                LogItemWeb {}
                LogItemNode {}
            }
        }
    }
}

#[component]
fn LogItemBash() -> Element {
    rsx! {
        div { class: "bg-white border border-outline-variant/10 rounded-lg p-4 hover:border-secondary/30 transition-colors shadow-sm",
            div { class: "flex items-start gap-4",
                div { class: "w-10 h-10 rounded-lg bg-surface-container flex items-center justify-center shrink-0",
                    span { class: "material-symbols-outlined text-on-surface-variant", "terminal" }
                }
                div { class: "flex-1 min-w-0",
                    div { class: "flex items-center justify-between mb-1",
                        div { class: "flex items-center gap-2",
                            span { class: "text-sm font-bold", "Bash Execution" }
                            span { class: "text-[10px] font-bold px-1.5 py-0.5 bg-tertiary-fixed text-on-tertiary-fixed rounded-full uppercase", "Success" }
                            span { class: "text-[10px] font-medium text-white bg-slate-700 px-1.5 py-0.5 rounded uppercase", "build" }
                        }
                        span { class: "text-[10px] font-medium text-on-surface-variant/60", "12:44:12 PM" }
                    }
                    p { class: "text-xs text-on-surface-variant leading-relaxed mb-3", "Executed `npm run build` in /projects/forge-ui. Optimization pass completed in 4.2s." }
                    div { class: "bg-slate-900 rounded-md p-3 font-mono text-[11px] text-blue-300 overflow-x-auto border border-slate-800",
                        span { class: "text-slate-500 mr-2", "$" }
                        "Asset size: 2.1MB (-14% reduction)"
                        br {}
                        span { class: "text-slate-500 mr-2", "$" }
                        "Gzip size: 452KB"
                    }
                }
            }
        }
    }
}

#[component]
fn LogItemDocument() -> Element {
    rsx! {
        div { class: "bg-white border border-outline-variant/10 rounded-lg p-4 hover:border-secondary/30 transition-colors shadow-sm",
            div { class: "flex items-start gap-4",
                div { class: "w-10 h-10 rounded-lg bg-secondary/5 border border-secondary/10 flex items-center justify-center shrink-0",
                    span { class: "material-symbols-outlined text-secondary", "description" }
                }
                div { class: "flex-1 min-w-0",
                    div { class: "flex items-center justify-between mb-1",
                        div { class: "flex items-center gap-2",
                            span { class: "text-sm font-bold", "Document Synthesis" }
                            span { class: "text-[10px] font-medium text-white bg-slate-700 px-1.5 py-0.5 rounded uppercase", "processing" }
                        }
                        span { class: "text-[10px] font-medium text-on-surface-variant/60", "12:15:08 PM" }
                    }
                    p { class: "text-xs text-on-surface-variant leading-relaxed", "Generated architecture summary for the 'Slate' design system documentation. 14 pages processed with 98% accuracy." }
                }
            }
        }
    }
}

#[component]
fn LogItemWeb() -> Element {
    rsx! {
        div { class: "bg-white border border-outline-variant/10 rounded-lg p-4 hover:border-secondary/30 transition-colors shadow-sm",
            div { class: "flex items-start gap-4",
                div { class: "w-10 h-10 rounded-lg bg-surface-container flex items-center justify-center shrink-0",
                    span { class: "material-symbols-outlined text-on-surface-variant", "language" }
                }
                div { class: "flex-1 min-w-0",
                    div { class: "flex items-center justify-between mb-1",
                        div { class: "flex items-center gap-2",
                            span { class: "text-sm font-bold", "Web Navigation" }
                            span { class: "text-[10px] font-bold px-1.5 py-0.5 bg-surface-container-high text-on-surface-variant rounded-full uppercase", "Complete" }
                        }
                        span { class: "text-[10px] font-medium text-on-surface-variant/60", "11:58:32 AM" }
                    }
                    p { class: "text-xs text-on-surface-variant leading-relaxed", "Browsed MDN Web Docs to verify CSS backdrop-filter compatibility." }
                }
            }
        }
    }
}

#[component]
fn LogItemNode() -> Element {
    rsx! {
        div { class: "bg-white border border-outline-variant/10 rounded-lg p-4 hover:border-secondary/30 transition-colors shadow-sm",
            div { class: "flex items-start gap-4",
                div { class: "w-10 h-10 rounded-lg bg-surface-container flex items-center justify-center shrink-0",
                    span { class: "material-symbols-outlined text-on-surface-variant", "settings_remote" }
                }
                div { class: "flex-1 min-w-0",
                    div { class: "flex items-center justify-between mb-1",
                        div { class: "flex items-center gap-2",
                            span { class: "text-sm font-bold", "Node Handshake" }
                            span { class: "text-[10px] font-bold px-1.5 py-0.5 bg-tertiary-fixed text-on-tertiary-fixed rounded-full uppercase", "Success" }
                        }
                        span { class: "text-[10px] font-medium text-on-surface-variant/60", "11:30:15 AM" }
                    }
                    p { class: "text-xs text-on-surface-variant leading-relaxed", "Established secure tunnel with Forge Cluster US-East. Latency: 12ms." }
                }
            }
        }
    }
}
