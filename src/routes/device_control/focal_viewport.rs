use dioxus::prelude::*;

#[component]
pub fn FocalViewport() -> Element {
    rsx! {
        // Context Header
        div { class: "px-6 py-4 flex justify-between items-end",
            div {
                div { class: "flex items-center gap-2 mb-0.5",
                    span { class: "label-sm text-[0.6875rem] uppercase tracking-widest text-on-surface-variant font-bold", "SESSION ID: AGENT-402-DELTA" }
                    span { class: "w-1.5 h-1.5 rounded-full bg-on-tertiary-container animate-pulse" }
                }
                h1 { class: "text-2xl font-bold tracking-tight text-on-surface", "Cross-App Control Center" }
            }
            div { class: "flex gap-3",
                button { class: "bg-surface-container-lowest text-on-surface-variant px-4 py-2 rounded-xl text-sm font-semibold flex items-center gap-2 border border-outline-variant/30 hover:bg-white transition-all",
                    span { class: "material-symbols-outlined text-lg", "history" }
                    "HISTORY"
                }
                button { class: "bg-secondary text-on-secondary px-5 py-2 rounded-xl text-sm font-semibold flex items-center gap-2 shadow-lg shadow-secondary/20 transition-all hover:brightness-105 active:scale-95",
                    span { class: "material-symbols-outlined text-lg", "play_arrow" }
                    "RESUME TASK"
                }
            }
        }
        // Focal Viewport
        div { class: "flex-1 px-6 pb-24 overflow-hidden",
            div { class: "h-full bg-white rounded-2xl shadow-2xl shadow-slate-200/50 flex flex-col border border-outline-variant/10 overflow-hidden relative group",
                // Browser bar
                div { class: "px-4 py-3 bg-surface-container flex items-center justify-between border-b border-outline-variant/20",
                    div { class: "flex items-center gap-4",
                        div { class: "flex gap-1.5",
                            div { class: "w-3 h-3 rounded-full bg-red-400/20 border border-red-400/40" }
                            div { class: "w-3 h-3 rounded-full bg-yellow-400/20 border border-yellow-400/40" }
                            div { class: "w-3 h-3 rounded-full bg-green-400/20 border border-green-400/40" }
                        }
                        div { class: "flex items-center gap-2 px-3 py-1 bg-white rounded-lg border border-outline-variant/20 shadow-sm",
                            span { class: "material-symbols-outlined text-sm text-outline", "lock" }
                            span { class: "text-xs text-on-surface-variant font-medium", "https://analytics.workspace.ai/quarterly-ledger-v2" }
                        }
                    }
                    div { class: "flex items-center gap-3",
                        span { class: "label-sm text-[0.625rem] text-secondary font-black uppercase tracking-widest bg-secondary/10 px-2 py-0.5 rounded", "LIVE" }
                        div { class: "px-2 py-1 bg-surface-container-high rounded text-on-surface-variant",
                            span { class: "material-symbols-outlined text-sm", "videocam" }
                        }
                    }
                }
                // Environment with AI overlays
                div { class: "flex-1 relative bg-slate-50 overflow-hidden",
                    img { alt: "Data Visualization Dashboard", class: "w-full h-full object-cover opacity-90 transition-all duration-700 group-hover:scale-[1.01]", src: "https://lh3.googleusercontent.com/aida-public/AB6AXuAdHbIiD8loCVOyGztO7V0ukUP7EXF9cN3zRxyAHHp4DIuqg20P16XtXgM0nKyxu2PdPvRKMheOuDSJLninJsRcDyAF--2JnAd2M0EgAMG3xqSQm2un-We94Ls1gzq7AlTc3paYogLBIpPVnjY-yJy08duWeCLPgqfztjdu7uYXiU0ce2IcA6US06jTkaf30u1gSdroYo_ulv4Qo-vCfL--TGwAkEx3Iw8ocFmDkDZnkm3jp9r_Jq_4zEMSr0t-2WHcT-gEVDiqJ3Y" }
                    // Detection overlays
                    div { class: "absolute inset-0 pointer-events-none p-12 bg-blue-900/5",
                        div { class: "absolute top-[20%] left-[55%] w-72 h-48 border-2 border-dashed border-secondary/80 rounded-lg shadow-[0_0_20px_rgba(0,88,190,0.2)]",
                            div { class: "absolute -top-6 left-0 px-2 py-1 bg-secondary text-white text-[9px] font-bold rounded-t shadow-lg uppercase tracking-widest", "OBJ: ANALYTIC_GRAPH_04" }
                            div { class: "absolute -bottom-6 right-0 px-2 py-1 bg-white text-secondary text-[9px] font-bold rounded-b border border-secondary shadow-lg", "99.8% CONFIDENCE" }
                            div { class: "absolute inset-0 bg-gradient-to-b from-transparent via-secondary/10 to-transparent animate-pulse h-1/2 top-1/4" }
                        }
                        div { class: "absolute bottom-[15%] left-[10%] w-96 h-32 border-2 border-dashed border-tertiary-fixed-dim/60 rounded-lg",
                            div { class: "absolute -top-6 left-0 px-2 py-1 bg-tertiary-fixed-dim text-on-tertiary text-[9px] font-bold rounded-t uppercase", "DATA_TABLE: Q3_REVENUE" }
                        }
                        // Agent Cursor
                        div { class: "absolute top-[45%] left-[70%] flex flex-col items-center gap-2 group/cursor",
                            div { class: "relative",
                                span { class: "material-symbols-outlined text-secondary text-4xl drop-shadow-[0_0_15px_rgba(0,88,190,0.5)] transform -rotate-12", "near_me" }
                                div { class: "absolute -top-1 -right-1 w-2 h-2 bg-red-500 rounded-full animate-ping" }
                            }
                            div { class: "px-2.5 py-1 glass-panel rounded-lg text-[9px] font-black text-secondary border border-secondary/40 shadow-xl uppercase tracking-tighter", "AGENT_INPUT: ACTIVE" }
                        }
                    }
                    // Interaction Toolbox
                    div { class: "absolute bottom-8 left-1/2 -translate-x-1/2 glass-panel p-2 rounded-2xl flex items-center gap-1 shadow-2xl border border-white/50",
                        button { class: "p-3 text-on-surface-variant hover:bg-white rounded-xl transition-all", span { class: "material-symbols-outlined", "pan_tool" } }
                        button { class: "p-3 text-on-surface-variant hover:bg-white rounded-xl transition-all", span { class: "material-symbols-outlined", "ads_click" } }
                        div { class: "w-px h-8 bg-outline-variant/30 mx-2" }
                        button { class: "p-3 bg-secondary text-on-secondary rounded-xl shadow-lg shadow-secondary/30 transition-all hover:scale-105 active:scale-95", span { class: "material-symbols-outlined", "keyboard" } }
                        button { class: "p-3 text-on-surface-variant hover:bg-white rounded-xl transition-all", span { class: "material-symbols-outlined", "code" } }
                    }
                }
            }
        }
    }
}
