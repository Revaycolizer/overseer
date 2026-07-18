use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn VoiceHeader(is_listening: bool, on_terminate: Callback<()>) -> Element {
    if !is_listening {
        return rsx! {};
    }

    rsx! {
        div { class: "bg-blue-50/40 border-b border-blue-100 px-6 py-4 flex flex-col gap-4 transition-all duration-300 animate-fade-in shrink-0",
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-6",
                    div { class: "flex items-center gap-2",
                        span { class: "w-2 h-2 rounded-full bg-blue-600 animate-ping" }
                        span { class: "text-[10px] font-black uppercase tracking-[0.2em] text-blue-600 font-mono", "System Active" }
                    }
                    h1 { class: "text-2xl font-black text-slate-800 tracking-tight", "Listening..." }
                }
                div { class: "flex items-center gap-4 text-xs font-bold text-slate-500 font-mono",
                    span {
                        "Uptime: "
                        span { class: "text-slate-800 font-bold", "14h 22m" }
                    }
                    span { class: "text-slate-300", "|" }
                    span {
                        "SESSION: "
                        span { class: "text-slate-800 font-bold", "AC-9921" }
                    }
                    button {
                        class: "ml-2 bg-red-50 hover:bg-red-100 text-red-600 border border-red-200/60 px-3.5 py-1.5 rounded-xl flex items-center gap-1.5 transition-all shadow-sm transform active:scale-95 text-[10px] font-black uppercase",
                        onclick: move |_| on_terminate.call(()),
                        MaterialIcon { name: "cancel", size: 12 }
                        "Terminate Task"
                    }
                }
            }

            // Sleek audio waveform visualizer lines
            div { class: "flex items-center justify-between gap-1 h-14 bg-white/70 border border-slate-200/50 rounded-2xl px-6 py-2 shadow-inner",
                for (i, &h) in [24, 42, 64, 28, 12, 54, 72, 36, 18, 58, 48, 16, 28, 62, 52, 24, 8, 48, 68, 30].iter().enumerate() {
                    div {
                        class: "w-1 bg-blue-600 rounded-full transition-all duration-300 animate-pulse",
                        style: "height: {h}%; animation-delay: {i as f32 * 0.05}s"
                    }
                }
            }
        }
    }
}

#[component]
pub fn OverseerBottomWidget(is_listening: bool, on_refresh: Callback<()>) -> Element {
    rsx! {
        div { class: "flex flex-col justify-between h-full space-y-4",
            div { class: "flex items-center gap-2",
                div { class: "w-2.5 h-2.5 rounded-full bg-blue-600 animate-pulse" }
                span { class: "text-xs font-black uppercase tracking-wider text-slate-800 font-mono", "Overseer Panel" }
            }
            // Dynamic waveform
            div { class: "flex items-center gap-1.5 h-12 bg-white border border-slate-200 rounded-xl px-4 shadow-sm justify-between",
                for (i, &h) in [16, 32, 48, 20, 10, 42, 54, 28, 14, 48].iter().enumerate() {
                    div {
                        class: "w-1 bg-blue-600 rounded-full animate-pulse",
                        style: "height: {h}%; animation-delay: {i as f32 * 0.08}s"
                    }
                }
            }
            p { class: "text-[10px] text-slate-500 font-medium italic leading-relaxed",
                if is_listening { "🎙️ Overseer is active and transcribing your voice in real-time..." }
                else { "🎙️ Say \"Hey Overseer\" to activate voice command panel." }
            }
        }
    }
}

#[component]
pub fn TelemetrySummary(uptime: String, session_id: String) -> Element {
    rsx! {
        div { class: "flex flex-col justify-between h-full space-y-4 text-xs font-mono",
            div { class: "flex justify-between items-center border-b border-slate-100 pb-2",
                span { class: "text-[9px] font-bold uppercase text-slate-400", "System Telemetry" }
                span { class: "bg-emerald-100 text-emerald-700 text-[8px] font-bold px-1.5 py-0.5 rounded tracking-wide uppercase", "Online" }
            }
            div { class: "space-y-1 text-slate-600 font-medium",
                p {
                    "Uptime: "
                    span { class: "text-slate-900 font-bold", "\"{uptime}\"" }
                }
                p {
                    "Session: "
                    span { class: "text-slate-900 font-bold", "\"{session_id}\"" }
                }
                p {
                    "Cluster: "
                    span { class: "text-slate-900 font-bold", "\"US-East Node\"" }
                }
            }
            p { class: "text-[8px] text-slate-400 font-bold uppercase tracking-wider", "Security Level: Shielded" }
        }
    }
}
