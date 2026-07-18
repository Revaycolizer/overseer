use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::Database;
use std::sync::Arc;
use super::supervisor_intervention::InterventionCard;

#[component]
pub fn SupervisorPanel(
    db: Arc<Database>,
    pending_action: String,
    pending_tool: String,
    approval_state: String,
    on_refresh: Callback<()>,
) -> Element {
    let mut pending_text = pending_action.clone();
    if pending_text.is_empty() {
        pending_text = "The agent detected that 12 cells in \"Global Ledger v2\" contain complex dependencies. Modification could impact downstream reports.".to_string();
    }

    rsx! {
        aside { class: "w-80 border-l border-slate-200 bg-white flex flex-col justify-between min-h-0 p-5 space-y-6 text-slate-700 font-sans",
            div { class: "space-y-6 overflow-y-auto custom-scrollbar flex-1 pr-1",
                div { class: "flex items-center justify-between border-b border-slate-200 pb-3",
                    div { class: "flex items-center gap-2",
                        MaterialIcon { name: "shield", size: 16, color: MaterialIconColor::Custom("#2563eb".into()) }
                        span { class: "text-xs font-black tracking-wider text-slate-800 uppercase font-mono", "Supervisor Panel" }
                    }
                    MaterialIcon { name: "verified_user", size: 14, color: MaterialIconColor::Custom("#2563eb".into()) }
                }
                div { class: "space-y-3",
                    div { class: "flex justify-between items-baseline",
                        span { class: "text-xs font-bold text-slate-500", "Agent Confidence" }
                        span { class: "text-2xl font-black text-blue-600 font-mono", "92%" }
                    }
                    div { class: "h-2 w-full bg-slate-100 rounded-full overflow-hidden border border-slate-200/50",
                        div { class: "h-full bg-gradient-to-r from-blue-600 to-indigo-500 rounded-full", style: "width: 92%" }
                    }
                    div { class: "grid grid-cols-3 gap-2 text-[8px] font-bold text-slate-500 font-mono uppercase tracking-wider text-center",
                        div { class: "bg-slate-50 border border-slate-200 rounded-lg p-1.5 shadow-sm",
                            p { class: "text-slate-800 text-xs font-black", "98%" }
                            p { class: "text-[7px] text-slate-400 mt-0.5", "Recognition" }
                        }
                        div { class: "bg-slate-50 border border-slate-200 rounded-lg p-1.5 shadow-sm",
                            p { class: "text-slate-800 text-xs font-black", "89%" }
                            p { class: "text-[7px] text-slate-400 mt-0.5", "Logic" }
                        }
                        div { class: "bg-slate-50 border border-slate-200 rounded-lg p-1.5 shadow-sm",
                            p { class: "text-slate-800 text-xs font-black", "94%" }
                            p { class: "text-[7px] text-slate-400 mt-0.5", "Safety" }
                        }
                    }
                }
                InterventionCard { db: db.clone(), pending_text, on_refresh: on_refresh.clone() }
                div { class: "space-y-3",
                    p { class: "text-[10px] font-black uppercase tracking-wider text-slate-400 font-mono", "Alternative Paths" }
                    div { class: "space-y-2 text-xs",
                        div { class: "flex items-center justify-between bg-white border border-slate-200 rounded-xl p-2.5 hover:bg-slate-50 transition-colors cursor-pointer shadow-sm",
                            span { class: "font-semibold text-slate-700", "Ignore formulas" }
                            span { class: "text-[10px] font-mono text-slate-400", "12% match" }
                        }
                        div { class: "flex items-center justify-between bg-white border border-slate-200 rounded-xl p-2.5 hover:bg-slate-50 transition-colors cursor-pointer shadow-sm",
                            span { class: "font-semibold text-slate-700", "Duplicate sheet" }
                            span { class: "text-[10px] font-mono text-blue-600 font-bold", "64% match" }
                        }
                    }
                }
            }
            div { class: "border-t border-slate-200 pt-4 flex items-center justify-between text-[10px] font-mono text-slate-500 uppercase tracking-widest bg-white -mx-5 -mb-5 px-5 py-4",
                p { span { "CPU: " } span { class: "text-slate-800 font-bold", "12%" } }
                p { span { "RAM: " } span { class: "text-slate-400 font-bold", "4.2GB" } }
                p { span { "NET: " } span { class: "text-slate-400 font-bold", "42ms" } }
            }
        }
    }
}
