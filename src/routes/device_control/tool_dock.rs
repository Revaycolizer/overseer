use dioxus::prelude::*;

#[component]
pub fn ToolDock() -> Element {
    rsx! {
        div { class: "absolute bottom-6 left-1/2 -translate-x-1/2 w-[640px] h-16 bg-white/90 backdrop-blur-xl border border-outline-variant/20 rounded-2xl shadow-2xl z-40 flex items-center px-4 gap-4",
            // Active Tool
            div { class: "flex-1 flex items-center gap-3 bg-secondary/5 border border-secondary/20 p-2 rounded-xl cursor-default",
                div { class: "w-10 h-10 bg-secondary/10 text-secondary rounded-lg flex items-center justify-center",
                    span { class: "material-symbols-outlined", "table_chart" }
                }
                div { class: "flex-1 min-w-0",
                    p { class: "text-[11px] font-black uppercase text-secondary", "Ledger Tool" }
                    p { class: "text-[10px] text-on-surface-variant truncate", "Quarterly-ledger-v2" }
                }
                span { class: "w-2 h-2 rounded-full bg-secondary" }
            }
            // Web Browser
            div { class: "flex-1 flex items-center gap-3 hover:bg-slate-50 p-2 rounded-xl cursor-pointer transition-colors group",
                div { class: "w-10 h-10 bg-surface-container-high text-outline group-hover:text-on-surface rounded-lg flex items-center justify-center transition-colors",
                    span { class: "material-symbols-outlined", "language" }
                }
                div { class: "flex-1 min-w-0",
                    p { class: "text-[11px] font-bold text-outline group-hover:text-on-surface", "Web Browser" }
                    p { class: "text-[10px] text-outline", "Search Mode" }
                }
            }
            // Outlook
            div { class: "flex-1 flex items-center gap-3 hover:bg-slate-50 p-2 rounded-xl cursor-pointer transition-colors group",
                div { class: "w-10 h-10 bg-surface-container-high text-outline group-hover:text-on-surface rounded-lg flex items-center justify-center transition-colors",
                    span { class: "material-symbols-outlined", "mail" }
                }
                div { class: "flex-1 min-w-0",
                    p { class: "text-[11px] font-bold text-outline group-hover:text-on-surface", "Outlook" }
                    p { class: "text-[10px] text-outline", "Queued (2)" }
                }
            }
        }
    }
}
