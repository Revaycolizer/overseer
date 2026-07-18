use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn WidgetsPanel(
    db: std::sync::Arc<crate::db::Database>,
    has_load: bool,
    has_gpu: bool,
    has_bg: bool,
    has_env: bool,
    pending_action: String,
    on_refresh: Callback<()>,
) -> Element {
    rsx! {
        aside { class: "w-80 border-l border-slate-200 bg-white flex flex-col min-h-0 overflow-y-auto custom-scrollbar p-5 space-y-6 text-slate-700 font-sans shrink-0",
            if has_load { SystemLoadWidget {} }
            if has_gpu { GpuPerformanceWidget {} }
            if has_bg { BackgroundProcessesWidget {} }
            if has_env { EnvironmentWidget {} }
        }
    }
}

#[component]
pub fn SystemLoadWidget() -> Element {
    rsx! {
        div { class: "space-y-3.5 shrink-0",
            div { class: "flex items-center justify-between border-b border-slate-100 pb-2",
                span { class: "text-xs font-black uppercase tracking-wider text-slate-800 font-mono", "System Load" }
                MaterialIcon { name: "trending_up", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#2563eb".into()) }
            }
            div { class: "grid grid-cols-2 gap-3",
                div { class: "bg-slate-50 border border-slate-200/60 rounded-xl p-3 shadow-sm",
                    div { class: "flex justify-between items-baseline mb-1.5",
                        span { class: "text-[9px] font-bold text-slate-400 uppercase font-mono", "CPU" }
                        span { class: "text-xs font-black text-blue-600 font-mono", "24%" }
                    }
                    div { class: "h-1.5 w-full bg-slate-200 rounded-full overflow-hidden",
                        div { class: "h-full bg-blue-600 rounded-full", style: "width: 24%" }
                    }
                }
                div { class: "bg-slate-50 border border-slate-200/60 rounded-xl p-3 shadow-sm",
                    div { class: "flex justify-between items-baseline mb-1.5",
                        span { class: "text-[9px] font-bold text-slate-400 uppercase font-mono", "RAM" }
                        span { class: "text-xs font-black text-blue-600 font-mono", "4.2GB" }
                    }
                    div { class: "h-1.5 w-full bg-slate-200 rounded-full overflow-hidden",
                        div { class: "h-full bg-blue-600 rounded-full", style: "width: 48%" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GpuPerformanceWidget() -> Element {
    rsx! {
        div { class: "bg-slate-950 border border-slate-900 rounded-2xl p-4.5 space-y-3.5 shadow-md text-white select-none relative overflow-hidden shrink-0",
            div { class: "flex justify-between items-baseline",
                span { class: "text-[9px] font-bold text-slate-500 uppercase tracking-widest font-mono", "GPU Performance" }
                span { class: "text-xs font-black text-blue-400 font-mono", "92% Load" }
            }
            div { class: "h-14 flex items-end gap-1.5 bg-slate-900/60 border border-slate-900 rounded-xl p-2 justify-between",
                for &h in [15, 30, 20, 45, 60, 85, 92, 70, 40, 60, 80, 95].iter() {
                    div { class: "flex-1 bg-gradient-to-t from-blue-600 to-cyan-400 rounded-t-sm", style: "height: {h}%" }
                }
            }
            div { class: "text-[8px] font-mono text-slate-500 uppercase tracking-wider text-center", "NVIDIA RTX 4090 Synthesis Unit" }
        }
    }
}

#[component]
pub fn BackgroundProcessesWidget() -> Element {
    rsx! {
        div { class: "space-y-4 shrink-0",
            div { class: "flex items-center justify-between border-b border-slate-100 pb-2",
                span { class: "text-xs font-black uppercase tracking-wider text-slate-800 font-mono", "Background Processes" }
            }
            div { class: "space-y-3",
                div { class: "bg-slate-50 border border-slate-200/60 rounded-xl p-3 shadow-sm space-y-2",
                    div { class: "flex justify-between items-center text-[10px]",
                        div { class: "flex items-center gap-1.5 font-bold text-slate-700",
                            MaterialIcon { name: "dns", size: 12, color: dioxus_material_icons::MaterialIconColor::Custom("#2563eb".into()) }
                            span { "Database Indexing" }
                        }
                        span { class: "font-mono font-bold text-blue-600", "88%" }
                    }
                    div { class: "h-1 w-full bg-slate-200 rounded-full overflow-hidden",
                        div { class: "h-full bg-blue-600 rounded-full", style: "width: 88%" }
                    }
                }
                div { class: "bg-slate-50 border border-slate-200/60 rounded-xl p-3 shadow-sm flex items-center justify-between",
                    div { class: "space-y-1",
                        div { class: "flex items-center gap-1.5 text-[10px] font-bold text-slate-700",
                            MaterialIcon { name: "sync", size: 12 }
                            span { "Cloud Sync" }
                        }
                        p { class: "text-[8px] text-slate-400 font-bold uppercase tracking-wider", "Awaiting Network" }
                    }
                    button { class: "text-slate-400 hover:text-slate-600 transition-colors", MaterialIcon { name: "pause_circle_outline", size: 18 } }
                }
                div { class: "bg-slate-50 border border-slate-200/60 rounded-xl p-3 shadow-sm flex items-center justify-between opacity-70",
                    div { class: "space-y-1",
                        div { class: "flex items-center gap-1.5 text-[10px] font-bold text-slate-700",
                            MaterialIcon { name: "security", size: 12 }
                            span { "Audit Service" }
                        }
                        p { class: "text-[8px] text-slate-400 font-bold uppercase tracking-wider", "Idle" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn EnvironmentWidget() -> Element {
    rsx! {
        div { class: "bg-slate-900 border border-slate-800 rounded-2xl overflow-hidden shadow-md relative text-white h-44 select-none group shrink-0",
            img {
                class: "absolute inset-0 w-full h-full object-cover opacity-60 group-hover:scale-105 transition-transform duration-700",
                src: "/home/revaycolizer/.gemini/antigravity/brain/90460d0b-eaec-4c67-91a3-92585ef4eaaf/foundry_rack_1779025937025.png"
            }
            div { class: "absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-950/40 to-transparent" }
            div { class: "absolute bottom-3 left-4 right-4 flex justify-between items-end",
                div { class: "space-y-1",
                    span { class: "text-[8px] font-bold text-slate-400 uppercase tracking-widest font-mono", "Environment" }
                    h4 { class: "text-xs font-black tracking-tight", "Foundry Node-01" }
                }
                div { class: "flex items-center gap-1.5 bg-emerald-500/10 border border-emerald-500/30 px-2 py-0.5 rounded-full text-[8px] font-bold text-emerald-400 uppercase tracking-wider font-mono",
                    span { class: "w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping" }
                    "Online"
                }
            }
        }
    }
}
