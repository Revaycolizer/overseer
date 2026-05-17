use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn ControlCenter(session_state: String) -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 bg-white border-r border-slate-200 p-6 space-y-6 overflow-y-auto custom-scrollbar",
            // Header bar of the center view
            div { class: "flex items-start justify-between flex-wrap gap-4",
                div { class: "space-y-1.5",
                    div { class: "flex items-center gap-2 text-[10px] font-black uppercase tracking-wider text-slate-400 font-mono",
                        span { "SESSION ID: AGENT-402-DELTA" }
                        span { class: "w-1.5 h-1.5 rounded-full bg-emerald-500" }
                    }
                    h2 { class: "text-2xl font-black text-slate-800 tracking-tight", "Cross-App Control Center" }
                }
                div { class: "flex gap-2.5",
                    button { class: "bg-white hover:bg-slate-50 text-slate-700 border border-slate-200 px-4 py-2 rounded-xl font-bold text-xs flex items-center gap-2 transition-all shadow-sm",
                        MaterialIcon { name: "history", size: 16 }
                        "HISTORY"
                    }
                    button { class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-xl font-bold text-xs flex items-center gap-2 transition-all shadow-md shadow-blue-500/10",
                        MaterialIcon { name: "play_arrow", size: 16 }
                        "RESUME TASK"
                    }
                }
            }

            // Remote Browser Window
            div { class: "flex-1 bg-slate-50 border border-slate-200 rounded-3xl overflow-hidden flex flex-col relative shadow-sm min-h-[480px]",
                // Window Header (Light Grey)
                div { class: "h-11 border-b border-slate-200 bg-slate-100/70 px-4 flex items-center justify-between text-slate-400",
                    div { class: "flex items-center gap-1.5",
                        div { class: "w-2.5 h-2.5 rounded-full bg-red-400/80" }
                        div { class: "w-2.5 h-2.5 rounded-full bg-yellow-400/80" }
                        div { class: "w-2.5 h-2.5 rounded-full bg-green-400/80" }
                    }
                    div { class: "flex-1 max-w-md mx-auto flex items-center bg-white border border-slate-200 rounded-xl px-3 py-1 font-mono text-[10px] text-slate-600 justify-between shadow-sm",
                        div { class: "flex items-center gap-2 truncate",
                            MaterialIcon { name: "lock", size: 10, color: dioxus_material_icons::MaterialIconColor::Custom("#10b981".into()) }
                            span { class: "truncate", "https://analytics.workspace.ai/quarterly-ledger-v2" }
                        }
                        span { class: "bg-blue-600 text-white text-[8px] font-bold px-1.5 py-0.5 rounded tracking-widest uppercase", "LIVE" }
                    }
                    MaterialIcon { name: "videocam", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#64748b".into()) }
                }

                // Window Viewport Content (Keep premium dark dashboard contrast!)
                div { class: "flex-1 relative flex flex-col items-center justify-center p-6 bg-slate-950 select-none overflow-hidden",
                    // SVG Grid Background
                    div { class: "absolute inset-0 bg-[linear-gradient(to_right,#1e293b_1px,transparent_1px),linear-gradient(to_bottom,#1e293b_1px,transparent_1px)] bg-[size:4rem_4rem] [mask-image:radial-gradient(ellipse_60%_50%_at_50%_0%,#000_70%,transparent_100%)] opacity-35" }

                    // Styled Premium Mockup Graphic
                    div { class: "w-full max-w-2xl bg-slate-900/90 border border-slate-800/80 rounded-2xl p-6 space-y-4 backdrop-blur shadow-2xl relative z-10",
                        div { class: "flex items-center justify-between border-b border-slate-800/60 pb-3",
                            h3 { class: "text-sm font-bold text-slate-300", "Q3 Revenue Dashboard" }
                            span { class: "text-[10px] font-mono text-slate-500", "99.8% Match Accuracy" }
                        }
                        div { class: "h-36 bg-slate-950/70 rounded-xl border border-slate-800/40 relative overflow-hidden flex items-end p-4 justify-between gap-1",
                            // Dynamic Bar Graph
                            for (i, &h) in [20, 40, 30, 60, 80, 95].iter().enumerate() {
                                div { class: "flex-1 bg-gradient-to-t from-blue-600 to-indigo-400 rounded-t transition-all", style: "height: {h}%" }
                            }
                            div { class: "absolute top-3 right-3 bg-indigo-500/10 border border-indigo-500/20 px-2 py-1 rounded text-[8px] font-mono text-indigo-400 font-bold uppercase", "OBJ: ANALYTIC_GRAPH_04" }
                        }
                        div { class: "flex justify-between items-center text-[10px] font-mono",
                            div { class: "flex items-center gap-1.5 text-emerald-400 bg-emerald-500/10 px-2 py-1 rounded border border-emerald-500/20",
                                span { class: "w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping" }
                                span { "AGENT_INPUT: ACTIVE" }
                            }
                            div { class: "text-slate-400 bg-slate-950 border border-slate-800 px-2 py-1 rounded", "DATA_TABLE: Q3_REVENUE" }
                        }
                    }

                    // Floating Interactive Toolbar overlay
                    div { class: "absolute bottom-6 bg-white border border-slate-200 rounded-2xl px-5 py-2.5 flex items-center gap-5 shadow-xl backdrop-blur-md z-20",
                        button { class: "text-slate-500 hover:text-red-500 transition-colors", MaterialIcon { name: "back_hand", size: 18 } }
                        button { class: "text-slate-500 hover:text-blue-500 transition-colors", MaterialIcon { name: "ads_click", size: 18 } }
                        button { class: "bg-blue-600 text-white p-2.5 rounded-xl shadow-md transition-transform transform active:scale-95", MaterialIcon { name: "keyboard", size: 18 } }
                        button { class: "text-slate-500 hover:text-indigo-500 transition-colors", MaterialIcon { name: "code", size: 18 } }
                    }
                }
            }

            // Bottom Horizontal switcher tabs (Clean light theme)
            div { class: "grid grid-cols-3 gap-3 text-xs font-bold",
                div { class: "bg-white border-t-2 border-blue-600 rounded-xl p-3 flex items-center justify-between text-slate-800 shadow-sm border border-slate-200",
                    div { class: "flex items-center gap-2",
                        MaterialIcon { name: "grid_view", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#2563eb".into()) }
                        span { "LEDGER TOOL" }
                    }
                    span { class: "w-1.5 h-1.5 rounded-full bg-blue-600" }
                }
                div { class: "bg-white hover:bg-slate-50 border border-slate-200 rounded-xl p-3 flex items-center gap-2 text-slate-500 cursor-pointer transition-colors shadow-sm",
                    MaterialIcon { name: "language", size: 16 }
                    span { "Web Browser" }
                }
                div { class: "bg-white hover:bg-slate-50 border border-slate-200 rounded-xl p-3 flex items-center justify-between text-slate-500 cursor-pointer transition-colors shadow-sm",
                    div { class: "flex items-center gap-2",
                        MaterialIcon { name: "mail_outline", size: 16 }
                        span { "Outlook" }
                    }
                    span { class: "bg-slate-100 text-slate-500 text-[9px] px-1.5 py-0.5 rounded border border-slate-200", "Queued 2" }
                }
            }
        }
    }
}
