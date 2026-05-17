use dioxus::prelude::*;

#[component]
pub fn SupervisorPanel() -> Element {
    rsx! {
        aside { class: "w-96 bg-white border-l border-outline-variant/10 p-6 flex flex-col gap-8 shadow-2xl z-50",
            div {
                div { class: "flex items-center justify-between mb-6",
                    h3 { class: "text-xs font-black uppercase tracking-[0.2em] text-outline", "Supervisor Panel" }
                    span { class: "material-symbols-outlined text-secondary text-lg", "verified_user" }
                }
                // Confidence Score
                div { class: "space-y-4 mb-8",
                    div { class: "flex items-end justify-between",
                        span { class: "text-sm font-bold text-on-surface", "Agent Confidence" }
                        span { class: "text-2xl font-black text-secondary", "92%" }
                    }
                    div { class: "relative h-2 bg-surface-container-high rounded-full overflow-hidden",
                        div { class: "absolute h-full bg-secondary w-[92%] rounded-full shadow-[0_0_10px_rgba(0,88,190,0.5)]" }
                        div { class: "absolute inset-0 bg-[linear-gradient(90deg,transparent_0%,rgba(255,255,255,0.3)_50%,transparent_100%)] animate-[shimmer_2s_infinite]" }
                    }
                    div { class: "grid grid-cols-3 gap-2 text-[9px] uppercase font-black text-outline tracking-wider",
                        div { class: "text-secondary", "RECOGNITION: 98%" }
                        div { "LOGIC: 89%" }
                        div { "SAFETY: 94%" }
                    }
                }
                // Decision Logic Card
                div { class: "p-5 bg-secondary/5 rounded-2xl border border-secondary/10 space-y-4",
                    div { class: "flex items-center gap-3",
                        div { class: "w-8 h-8 rounded-full bg-secondary/10 flex items-center justify-center text-secondary",
                            span { class: "material-symbols-outlined text-lg", "troubleshoot" }
                        }
                        h4 { class: "text-xs font-bold text-on-surface uppercase tracking-tight", "Active Intervention Required" }
                    }
                    p { class: "text-[11px] text-on-surface-variant leading-relaxed",
                        "The agent detected that "
                        span { class: "font-bold text-secondary", "12 cells" }
                        " in "
                        span { class: "italic", "\"Global Ledger v2\"" }
                        " contain complex dependencies. Modification could impact downstream reports."
                    }
                    div { class: "flex flex-col gap-2 pt-2",
                        button { class: "w-full py-3 bg-secondary text-on-secondary text-xs font-black rounded-xl uppercase tracking-widest shadow-lg shadow-secondary/20 hover:brightness-105 active:scale-[0.98] transition-all",
                            "Approve Modification"
                        }
                        button { class: "w-full py-3 bg-white text-on-surface text-xs font-black rounded-xl uppercase tracking-widest border border-outline-variant/30 hover:bg-slate-50 transition-all",
                            "Detailed Review"
                        }
                    }
                }
            }
            // Alternative Paths
            div { class: "mt-auto space-y-4",
                h4 { class: "text-[10px] font-black uppercase text-outline tracking-widest border-b border-outline-variant/10 pb-2", "Alternative Paths" }
                div { class: "space-y-2",
                    div { class: "flex items-center justify-between p-2 rounded-lg hover:bg-slate-50 cursor-pointer",
                        span { class: "text-[11px] text-on-surface-variant", "Ignore formulas" }
                        span { class: "text-[10px] font-mono text-outline", "12% match" }
                    }
                    div { class: "flex items-center justify-between p-2 rounded-lg hover:bg-slate-50 cursor-pointer",
                        span { class: "text-[11px] text-on-surface-variant", "Duplicate sheet" }
                        span { class: "text-[10px] font-mono text-secondary", "64% match" }
                    }
                }
            }
        }
    }
}
