use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::Database;
use std::sync::Arc;

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
                // Header
                div { class: "flex items-center justify-between border-b border-slate-200 pb-3",
                    div { class: "flex items-center gap-2",
                        MaterialIcon { name: "shield", size: 16, color: MaterialIconColor::Custom("#2563eb".into()) }
                        span { class: "text-xs font-black tracking-wider text-slate-800 uppercase font-mono", "Supervisor Panel" }
                    }
                    MaterialIcon { name: "verified_user", size: 14, color: MaterialIconColor::Custom("#2563eb".into()) }
                }

                // Agent Confidence Section
                div { class: "space-y-3",
                    div { class: "flex justify-between items-baseline",
                        span { class: "text-xs font-bold text-slate-500", "Agent Confidence" }
                        span { class: "text-2xl font-black text-blue-600 font-mono", "92%" }
                    }
                    // Progress Bar
                    div { class: "h-2 w-full bg-slate-100 rounded-full overflow-hidden border border-slate-200/50",
                        div { class: "h-full bg-gradient-to-r from-blue-600 to-indigo-500 rounded-full", style: "width: 92%" }
                    }
                    // Metrics Row
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

                // Active Intervention Card (Light Blue matching the mockup exactly!)
                div { class: "bg-blue-50/70 border border-blue-100 rounded-2xl p-4.5 space-y-4 shadow-sm",
                    div { class: "flex items-center gap-2 text-blue-600",
                        MaterialIcon { name: "error_outline", size: 16 }
                        span { class: "text-[10px] font-black uppercase tracking-wider font-mono", "Active Intervention Required" }
                    }
                    p { class: "text-xs text-blue-900 leading-relaxed font-medium", "{pending_text}" }
                    div { class: "flex flex-col gap-2.5",
                        button {
                            class: "w-full bg-blue-600 hover:bg-blue-750 text-white py-3 rounded-xl font-bold text-xs shadow-md shadow-blue-500/10 transition-all transform active:scale-98 uppercase tracking-wider",
                            onclick: {
                                let db = db.clone();
                                move |_| {
                                    let db = db.clone();
                                    spawn(async move {
                                        let _ = crate::backend::approve_pending_computer_action(&db).await;
                                        on_refresh.call(());
                                    });
                                }
                            },
                            "Approve Modification"
                        }
                        button {
                            class: "w-full bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 py-3 rounded-xl font-bold text-xs transition-all transform active:scale-98 uppercase tracking-wider shadow-sm",
                            onclick: {
                                let db = db.clone();
                                move |_| {
                                    let db = db.clone();
                                    spawn(async move {
                                        let _ = crate::backend::reject_pending_computer_action(&db).await;
                                        on_refresh.call(());
                                    });
                                }
                            },
                            "Detailed Review"
                        }
                    }
                }

                // Alternative Paths Section
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

            // Bottom System Telemetry bar
            div { class: "border-t border-slate-200 pt-4 flex items-center justify-between text-[10px] font-mono text-slate-500 uppercase tracking-widest bg-white -mx-5 -mb-5 px-5 py-4",
                p {
                    "CPU: "
                    span { class: "text-slate-800 font-bold", "12%" }
                }
                p {
                    "RAM: "
                    span { class: "text-slate-400 font-bold", "4.2GB" }
                }
                p {
                    "NET: "
                    span { class: "text-slate-400 font-bold", "42ms" }
                }
            }
        }
    }
}
