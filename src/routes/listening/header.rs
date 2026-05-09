use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use super::waveform::Waveform;

#[component]
pub fn Header() -> Element {
    rsx! {
        section { class: "p-6 border-b border-outline-variant/10 bg-white/50",
            div { class: "flex items-center justify-between mb-4",
                div { class: "flex flex-col",
                    div { class: "flex items-center gap-2 mb-1",
                        span { class: "flex h-2 w-2 rounded-full bg-secondary animate-ping" }
                        span { class: "text-[10px] uppercase tracking-[0.2em] font-bold text-secondary", "System Active" }
                    }
                    h1 { class: "text-2xl font-extrabold tracking-tight text-on-surface", "Listening..." }
                }
                div { class: "flex items-center gap-4",
                    div { class: "text-right",
                        span { class: "block text-xs font-bold text-on-surface", "Uptime: 14h 22m" }
                        span { class: "block text-[10px] text-on-surface-variant uppercase tracking-wider", "Session: AC-9921" }
                    }
                    button { class: "bg-red-50 text-red-600 border border-red-100 px-4 py-2 rounded-lg font-bold text-xs flex items-center gap-2 hover:bg-red-100 transition-all",
                        MaterialIcon { name: "stop_circle", size: 16 }
                        "Terminate Task"
                    }
                }
            }
            Waveform {}
        }
    }
}
