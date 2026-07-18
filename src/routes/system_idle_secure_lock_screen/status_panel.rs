use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn StatusPanel() -> Element {
    rsx! {
        div { class: "flex flex-col items-center gap-12",
            div { class: "bg-white/80 backdrop-blur-xl px-12 py-8 rounded-2xl shadow-xl shadow-slate-200/50 border border-slate-100 flex flex-col items-center gap-4",
                div { class: "w-14 h-14 bg-blue-50 rounded-full flex items-center justify-center border border-blue-100/50",
                    MaterialIcon {
                        name: "nights_stay",
                        color: MaterialIconColor::Custom("#3b82f6".into()),
                        size: 24,
                    }
                }
                div { class: "space-y-1.5",
                    p { class: "text-lg font-bold text-slate-800 tracking-tight", "System Idle" }
                    p { class: "text-xs text-slate-500 font-medium", "Monitoring environmental inputs..." }
                }
            }
 
            div { class: "pt-4",
                div { class: "flex items-center gap-3 bg-slate-900 px-10 py-4 rounded-full text-white shadow-2xl shadow-slate-900/20",
                    MaterialIcon {
                        name: "mic",
                        color: MaterialIconColor::Light,
                        size: 20,
                    }
                    span { class: "text-sm font-medium tracking-wide",
                        "Say "
                        span { class: "text-blue-400 font-bold", "\"Hey Overseer\"" }
                        " to wake"
                    }
                }
            }
        }
    }
}
