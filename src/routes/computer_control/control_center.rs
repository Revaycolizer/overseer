use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn ControlCenter(session_state: String) -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 bg-slate-900 border-r border-slate-800 p-6 space-y-6 overflow-y-auto custom-scrollbar",
            // Header bar of the center view
            div { class: "flex items-start justify-between flex-wrap gap-4",
                div { class: "space-y-1.5",
                    div { class: "flex items-center gap-2 text-[10px] font-black uppercase tracking-wider text-slate-500 font-mono",
                        span { "SESSION ID: AGENT-402-DELTA" }
                        span { class: "w-1.5 h-1.5 rounded-full bg-emerald-500" }
                    }
                    h2 { class: "text-2xl font-black text-white tracking-tight", "Cross-App Control Center" }
                }
                div { class: "flex gap-2.5",
                    button { class: "bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700/80 px-4 py-2 rounded-xl font-bold text-xs flex items-center gap-2 transition-all",
                        MaterialIcon { name: "history", size: 16 }
                        "HISTORY"
                    }
                    button { class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-xl font-bold text-xs flex items-center gap-2 transition-all shadow-lg shadow-blue-500/10",
                        MaterialIcon { name: "play_arrow", size: 16 }
                        "RESUME TASK"
                    }
                }
            }

            // Remote/Mock Browser Window
            div { class: "flex-1 bg-slate-950 border border-slate-800 rounded-3xl overflow-hidden flex flex-col relative group shadow-2xl min-h-[480px]",
                // Window Header
                div { class: "h-11 border-b border-slate-800/80 bg-slate-900/60 px-4 flex items-center justify-between text-slate-500",
                    div { class: "flex items-center gap-1.5",
                        div { class: "w-2.5 h-2.5 rounded-full bg-red-500/70" }
                        div { class: "w-2.5 h-2.5 rounded-full bg-yellow-500/70" }
                        div { class: "w-2.5 h-2.5 rounded-full bg-green-500/70" }
                    }
                    div { class: "flex-1 max-w-md mx-auto flex items-center bg-slate-950 border border-slate-800 rounded-xl px-3 py-1 font-mono text-[10px] text-slate-400 justify-between",
                        div { class: "flex items-center gap-2 truncate",
                            MaterialIcon { name: "lock", size: 10, color: dioxus_material_icons::MaterialIconColor::Custom("#10b981".into()) }
                            span { class: "truncate", "https://analytics.workspace.ai/quarterly-ledger-v2" }
                        }
                        span { class: "bg-red-500/10 text-red-400 text-[8px] font-bold px-1 rounded tracking-widest", "LIVE" }
                    }
                    MaterialIcon { name: "videocam", size: 16 }
                }

                // Window Viewport Content
                div { class: "flex-1 relative flex flex-col items-center justify-center p-6 bg-slate-900/40 select-none overflow-hidden",
                    // SVG Grid Background
                    div { class: "absolute inset-0 bg-[linear-gradient(to_right,#1e293b_1px,transparent_1px),linear-gradient(to_bottom,#1e293b_1px,transparent_1px)] bg-[size:4rem_4rem] [mask-image:radial-gradient(ellipse_60%_50%_at_50%_0%,#000_70%,transparent_100%)] opacity-30" }

                    // Styled Premium Mockup Graphic
                    div { class: "w-full max-w-2xl bg-slate-950/80 border border-slate-800/80 rounded-2xl p-6 space-y-4 backdrop-blur shadow-2xl relative",
                        div { class: "flex items-center justify-between border-b border-slate-800/60 pb-3",
                            h3 { class: "text-sm font-bold text-slate-300", "Q3 Revenue Dashboard" }
                            span { class: "text-[10px] font-mono text-slate-500", "99.8% Match Accuracy" }
                        }
                        div { class: "h-36 bg-slate-900/50 rounded-xl border border-slate-800/40 relative overflow-hidden flex items-end p-4 justify-between gap-1",
                            // Dynamic Bar Graph
                            for (i, &h) in [20, 40, 30, 60, 80, 95].iter().enumerate() {
                                div { class: "flex-1 bg-gradient-to-t from-blue-600 to-indigo-400 rounded-t transition-all", style: "height: {h}%" }
                            }
                            div { class: "absolute top-3 right-3 bg-indigo-500/10 border border-indigo-500/20 px-2 py-1 rounded text-[8px] font-mono text-indigo-400 font-bold uppercase", "OBJ: ANALYTIC_GRAPH_04" }
                        }
                        div { class: "flex justify-between items-center text-[10px] font-mono",
                            div { class: "flex items-center gap-1.5 text-emerald-400 bg-emerald-500/10 px-2 py-1 rounded border border-emerald-500/20",
                                span { class: "w-1 h-1 rounded-full bg-emerald-400 animate-ping" }
                                span { "AGENT_INPUT: ACTIVE" }
                            }
                            div { class: "text-slate-400 bg-slate-900 border border-slate-800 px-2 py-1 rounded", "DATA_TABLE: Q3_REVENUE" }
                        }
                    }

                    // Floating Interactive Toolbar overlay
                    div { class: "absolute bottom-6 bg-slate-950/90 border border-slate-800/80 rounded-2xl px-5 py-2.5 flex items-center gap-5 shadow-2xl backdrop-blur-md",
                        button { class: "text-slate-400 hover:text-red-400 transition-colors", MaterialIcon { name: "back_hand", size: 18 } }
                        button { class: "text-slate-400 hover:text-blue-400 transition-colors", MaterialIcon { name: "ads_click", size: 18 } }
                        button { class: "bg-blue-600 text-white p-2 rounded-xl shadow-lg transition-transform transform active:scale-95", MaterialIcon { name: "keyboard", size: 18 } }
                        button { class: "text-slate-400 hover:text-indigo-400 transition-colors", MaterialIcon { name: "code", size: 18 } }
                    }
                }
            }

            // Bottom Horizontal tool switcher tabs
            div { class: "grid grid-cols-3 gap-3 text-xs font-bold",
                div { class: "bg-slate-950 border-t-2 border-blue-500 rounded-xl p-3 flex items-center justify-between text-white cursor-pointer shadow-md",
                    div { class: "flex items-center gap-2",
                        MaterialIcon { name: "grid_view", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#3b82f6".into()) }
                        span { "LEDGER TOOL" }
                    }
                    span { class: "w-1.5 h-1.5 rounded-full bg-blue-500" }
                }
                div { class: "bg-slate-950/40 border border-slate-800/80 hover:bg-slate-850 rounded-xl p-3 flex items-center gap-2 text-slate-400 cursor-pointer transition-colors",
                    MaterialIcon { name: "language", size: 16 }
                    span { "Web Browser" }
                }
                div { class: "bg-slate-950/40 border border-slate-800/80 hover:bg-slate-850 rounded-xl p-3 flex items-center justify-between text-slate-400 cursor-pointer transition-colors",
                    div { class: "flex items-center gap-2",
                        MaterialIcon { name: "mail_outline", size: 16 }
                        span { "Outlook" }
                    }
                    span { class: "bg-slate-800 text-slate-400 text-[9px] px-1.5 py-0.5 rounded", "Queued 2" }
                }
            }
        }
    }
}
